use super::*;
use app::constants::*;
use app::Event;
use backend::messagebus::{Inbox, Message, PubSub, ReceiveDrain, Whiteboard};
use backend::obj;
use backend::obj::*;
use backend::world;
use backend::world::agent;
use backend::world::segment;
use backend::world::segment::Intent;
use backend::world::segment::PilotRotation;
use cgmath::InnerSpace;
use core::geometry::Transform;
use core::geometry::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;
use wrapped2d::b2;
use wrapped2d::dynamics::world::callbacks::ContactAccess;
use wrapped2d::user_data::*;

struct AgentData;

impl UserDataTypes for AgentData {
	type BodyData = agent::Key;
	type JointData = ();
	type FixtureData = agent::Key;
}

type ContactSet = Rc<RefCell<HashMap<agent::Key, agent::Key>>>;

pub struct PhysicsSystem {
	world: b2::World<AgentData>,
	initial_extent: Rect,
	inbox: Option<Inbox>,
	handles: HashMap<agent::Key, b2::BodyHandle>,
	touched: ContactSet,
	picked: HashSet<Id>,
}

#[allow(unused)]
enum BodyUpdate {
	Transform(b2::Vec2, f32),
	Torque(f32),
	AngularImpulse(f32),
	Force(b2::Vec2, b2::Vec2),
	LinearImpulse(b2::Vec2, b2::Vec2),
}

struct JointRef<'a> {
	refs: agent::Key,
	handle: b2::BodyHandle,
	growing_radius: f32,
	rest_angle: f32,
	mesh: &'a obj::Mesh,
	flags: segment::Flags,
	attachment: Option<segment::Attachment>,
}

impl System for PhysicsSystem {
	fn attach(&mut self, bus: &mut PubSub) {
		self.inbox = Some(bus.subscribe(Box::new(|m| matches!(*m, Message::Event(Event::PickMinion(_))))));
	}

	fn init(&mut self, world: &world::World) {
		self.initial_extent = world.extent;
		self.init_extent();
	}

	fn clear(&mut self) {
		for i in &self.inbox {
			i.drain();
		}
		self.touched.borrow_mut().clear();
		self.handles.clear();
		self.picked.clear();
		self.world = Self::new_world(self.touched.clone());
		self.init_extent();
	}

	fn register(&mut self, agent: &world::agent::Agent) {
		// build fixtures
		let joint_refs = PhysicsSystem::build_fixtures(&mut self.world, &agent);
		// and then assemble them with joints
		PhysicsSystem::build_joints(&mut self.world, &joint_refs);
		// record them
		for JointRef { refs, handle, .. } in joint_refs {
			self.handles.insert(refs, handle);
		}
	}

	fn unregister(&mut self, agent: &world::agent::Agent) {
		let object_id = agent.id();
		let segments = agent.segments();
		for segment in segments {
			let refs = agent::Key::with_segment(object_id, segment.index);
			if let Some(handle) = self.handles.remove(&refs) {
				self.world.destroy_body(handle);
			}
		}
	}

	fn import(&mut self, world: &world::World) {
		let messages = match self.inbox {
			Some(ref drain) => drain.drain(),
			None => Vec::new(),
		};
		self.picked.clear();
		for message in messages {
			if let Message::Event(Event::PickMinion(position)) = message {
				let picked = self.pick(position);
				if let Some(picked_id) = picked {
					self.picked.insert(picked_id);
				}
			}
		}
		for agent in world.agents(agent::AgentType::Minion).values() {
			if agent.state.growth() > 0. {
				self.refresh_registration(agent)
			}
		}
	}

	fn update(&mut self, state: &dyn world::AgentState, dt_sec: Seconds) {
		use self::BodyUpdate::*;
		let mut dynamic_updates = Vec::new();
		let dt: f32 = dt_sec.into();
		#[inline]
		fn to_vec2(v: Position) -> b2::Vec2 { PhysicsSystem::p2v(v) };
		#[inline]
		fn from_vec2(v: b2::Vec2) -> Position { PhysicsSystem::v2p(v) };
		for (h, b) in self.world.bodies() {
			let body = b.borrow();
			let center = *(*body).world_center();
			let key = (*body).user_data();
			if let Some(segment) = state.agent(key.agent_id).and_then(|c| c.segment(key.segment_index)) {
				match segment.state.intent {
					Intent::Move(force) => dynamic_updates.push((h, Force(center, to_vec2(force)))),
					Intent::Brake(force) => {
						let linear_velocity = from_vec2(*(*body).linear_velocity());
						let comp = force.dot(linear_velocity);
						if comp < 0. {
							dynamic_updates.push((h, Force(center, to_vec2(force))));
						}
					}
					Intent::PilotTo(force, ref target_angle) => {
						if let Some(force) = force {
							let linear_velocity = from_vec2(*(*body).linear_velocity());
							let speed2 = linear_velocity.magnitude2();
							let drag_factor = (1. - speed2 * DRAG_COEFFICIENT).min(1.).max(0.);
							dynamic_updates.push((h, Force(center, to_vec2(force * drag_factor))));
						}
						match *target_angle {
							PilotRotation::LookAt(target) => {
								let look_at_vector = target - from_vec2(center);
								let target_angle = f32::atan2(-look_at_vector.x, look_at_vector.y);
								dynamic_updates.push((h, Transform(*(*body).position(), target_angle)));
							}
							PilotRotation::Orientation(direction) => {
								let target_angle = f32::atan2(-direction.x, direction.y);
								dynamic_updates.push((h, Transform(*(*body).position(), target_angle)));
							}
							PilotRotation::Turn(angle) => {
								dynamic_updates.push((h, Torque(angle)));
							}
							PilotRotation::FromVelocity => {
								let linear_velocity = from_vec2(*(*body).linear_velocity());
								let target_angle = f32::atan2(-linear_velocity.x, linear_velocity.y);
								dynamic_updates.push((h, Transform(*(*body).position(), target_angle)));
							}
							PilotRotation::None => {}
							//TODO: try physics!
							//let angle = (*body).angle();
							//let norm_diff = math::normalize_rad(target_angle - angle);
							//body_updates.push((h, Torque(norm_diff * COMPASS_SPRING_POWER)))
							//torques.push((h, norm_diff * COMPASS_SPRING_POWER));
						}
					}
					Intent::RunAway(impulse) => dynamic_updates.push((h, LinearImpulse(center, to_vec2(impulse * dt)))),
					_ => {}
				}
			}
		}

		for (h, update) in dynamic_updates {
			let b = &mut self.world.body_mut(h);
			match update {
				BodyUpdate::Torque(torque) => b.apply_torque(torque, true),
				BodyUpdate::AngularImpulse(impulse) => b.apply_angular_impulse(impulse, true),
				BodyUpdate::Force(application_point, force) => b.apply_force(&force, &application_point, true),
				BodyUpdate::LinearImpulse(application_point, impulse) =>
					b.apply_linear_impulse(&impulse, &application_point, true),
				BodyUpdate::Transform(translation, rotation) => b.set_transform(&translation, rotation),
			}
		}
		self.world.step(dt, 8, 3);
	}

	fn export(&self, world: &mut world::World, outbox: &dyn Outbox) {
		for (_, b) in self.world.bodies() {
			let body = b.borrow();
			let position = (*body).position();
			let angle = (*body).angle();
			let velocity = (*body).linear_velocity();
			let spin = (*body).angular_velocity();
			let key = (*body).user_data();

			if let Some(agent) = world.agent_mut(key.agent_id) {
				if let Some(segment) = agent.segment_mut(key.segment_index) {
					segment.transform_to(Transform::from_components(position.x, position.y, angle));
					segment.motion_to(Motion::from_components(velocity.x, velocity.y, spin));
					segment.state.last_touched = self.touched.borrow().get(key).cloned();
				}
			}
		}
		for (_, agent) in world.agents_mut(agent::AgentType::Minion).iter_mut() {
			agent.state.reset_growth()
		}
		for id in &self.picked {
			outbox.post(Event::SelectMinion(*id).into());
		}
		self.touched.borrow_mut().clear();
	}
}

impl Default for PhysicsSystem {
	fn default() -> Self {
		let touched = Rc::new(RefCell::new(HashMap::new()));
		PhysicsSystem {
			inbox: None,
			initial_extent: Rect::default(),
			world: Self::new_world(touched.clone()),
			handles: HashMap::with_capacity(5000),
			picked: HashSet::with_capacity(100),
			touched,
		}
	}
}

impl PhysicsSystem {
	fn p2v(p: Position) -> b2::Vec2 { b2::Vec2 { x: p.x, y: p.y } }

	fn pr2v(p: Position, radius: f32) -> b2::Vec2 { b2::Vec2 { x: p.x * radius, y: p.y * radius } }

	fn v2p(p: b2::Vec2) -> Position { Position::new(p.x, p.y) }

	fn init_extent(&mut self) {
		let extent = self.initial_extent;
		let mut f_def = b2::FixtureDef::new();
		let mut b_def = b2::BodyDef::new();
		b_def.body_type = b2::BodyType::Static;
		let refs = agent::Key::with_id(0xFFFF_FFFFusize);
		let handle = self.world.create_body_with(&b_def, refs);

		let mut rect = b2::ChainShape::new();
		rect.create_loop(&[
			Self::p2v(extent.bottom_left()),
			Self::p2v(extent.bottom_right()),
			Self::p2v(extent.top_right()),
			Self::p2v(extent.top_left()),
		]);

		self.world.body_mut(handle).create_fixture_with(&rect, &mut f_def, refs);
	}

	fn refresh_registration(&mut self, agent: &world::agent::Agent) {
		self.unregister(agent);
		self.register(agent);
	}

	#[allow(clippy::too_many_arguments)]
	fn build_fixture_for_segment(
		world: &mut b2::World<AgentData>,
		handle: b2::BodyHandle,
		maturity: f32,
		object_id: obj::Id,
		segment_index: usize,
		refs: agent::Key,
		f_def: &mut b2::FixtureDef,
		mesh: &Mesh,
	) {
		fn make_circle_shape(
			world: &mut b2::World<AgentData>,
			handle: b2::BodyHandle,
			grown_radius: f32,
			f_def: &mut b2::FixtureDef,
			refs: agent::Key,
		) {
			let mut circle_shape = b2::CircleShape::new();
			circle_shape.set_radius(grown_radius);
			world.body_mut(handle).create_fixture_with(&circle_shape, f_def, refs);
		}

		fn make_safe_poly_shape(
			world: &mut b2::World<AgentData>,
			handle: b2::BodyHandle,
			grown_radius: f32,
			vertices: &[b2::Vec2],
			f_def: &mut b2::FixtureDef,
			refs: agent::Key,
		) {
			// from box2d code, checks unique vertices
			let mut dupes = 0;
			for (i, v1) in vertices.iter().enumerate() {
				for v2 in &vertices[(i + 1)..] {
					let d2 = (v2 - v1).sqr_norm();
					if d2 < 0.5f32 * B2_LINEAR_SLOP {
						dupes += 1;
					}
				}
			}
			// if we have enough non-degenerate vertices, we hand them over to b2d
			// TOOD: optimize (don't send dupes)
			if vertices.len() - dupes > 2 {
				let mut poly = b2::PolygonShape::new();
				poly.set(vertices);
				world.body_mut(handle).create_fixture_with(&poly, f_def, refs);
			} else {
				// tiny circle, failover
				make_circle_shape(world, handle, grown_radius, f_def, refs);
			}
		}

		match mesh.shape {
			obj::Shape::Ball { radius } => {
				make_circle_shape(world, handle, radius * maturity, f_def, refs);
			}
			obj::Shape::Box { radius, ratio } => {
				let mut rect_shape = b2::PolygonShape::new();
				rect_shape.set_as_box(radius * ratio, radius * maturity);
				world.body_mut(handle).create_fixture_with(&rect_shape, f_def, refs);
			}
			obj::Shape::Poly { radius, n, .. } => {
				let grown_radius = radius * maturity;
				let p = &mesh.vertices;
				let offset = if n < 0 { 1 } else { 0 };
				let mut vertices = Vec::new();
				for i in 0..n.abs() {
					vertices.push(Self::pr2v(p[2 * i as usize + offset], grown_radius));
				}
				make_safe_poly_shape(world, handle, grown_radius, vertices.as_slice(), f_def, refs);
			}
			obj::Shape::Star { radius, n, .. } => {
				let grown_radius = radius * maturity;
				if grown_radius < 0.05 * f32::from(n) {
					make_circle_shape(world, handle, grown_radius, f_def, refs);
				} else {
					let p = &mesh.vertices;
					for i in 0..n {
						let i1 = (i * 2 + 1) as usize;
						let i2 = (i * 2) as usize;
						let i3 = ((i * 2 + (n * 2) - 1) % (n * 2)) as usize;
						let (p1, p2, p3) = match mesh.winding() {
							obj::Winding::CW => (p[i1], p[i2], p[i3]),
							obj::Winding::CCW => (p[i1], p[i3], p[i2]),
						};
						let quad_vertices = &[
							b2::Vec2 { x: 0., y: 0. },
							Self::pr2v(p1, grown_radius),
							Self::pr2v(p2, grown_radius),
							Self::pr2v(p3, grown_radius),
						];
						let refs = agent::Key::with_bone(object_id, segment_index as u8, i as u8);
						make_safe_poly_shape(world, handle, grown_radius, quad_vertices, f_def, refs);
					}
				}
			}
			obj::Shape::Triangle { radius, .. } => {
				let grown_radius = radius * maturity;
				let p = &mesh.vertices;
				let (p1, p2, p3) = match mesh.winding() {
					obj::Winding::CW => (p[0], p[2], p[1]),
					obj::Winding::CCW => (p[0], p[1], p[2]),
				};
				let tri_vertices =
					&[Self::pr2v(p1, grown_radius), Self::pr2v(p2, grown_radius), Self::pr2v(p3, grown_radius)];
				make_safe_poly_shape(world, handle, grown_radius, tri_vertices, f_def, refs);
			}
		};
	}

	fn build_fixtures<'a>(world: &mut b2::World<AgentData>, agent: &'a world::agent::Agent) -> Vec<JointRef<'a>> {
		let object_id = agent.id();
		let segments = agent.segments();
		segments
			.iter()
			.enumerate()
			.map(|(segment_index, segment)| {
				let material = segment.material();
				let growing_radius = segment.growing_radius();
				let mut f_def = b2::FixtureDef::new();
				f_def.density = material.density;
				f_def.restitution = material.restitution;
				f_def.friction = material.friction;

				let transform = segment.transform();
				let mut b_def = b2::BodyDef::new();
				b_def.body_type = b2::BodyType::Dynamic;
				b_def.linear_damping = material.linear_damping;
				b_def.angular_damping = material.angular_damping;
				b_def.angle = transform.angle;
				b_def.position = Self::pr2v(transform.position, 1.);
				b_def.linear_velocity = Self::pr2v(segment.motion.velocity, 1.);
				b_def.angular_velocity = segment.motion.spin;
				let refs = agent::Key::with_segment(object_id, segment_index as u8);
				let handle = world.create_body_with(&b_def, refs);
				let mesh = segment.mesh();
				let rest_angle = segment.rest_angle;
				Self::build_fixture_for_segment(
					world,
					handle,
					segment.state.maturity(),
					object_id,
					segment_index,
					refs,
					&mut f_def,
					mesh,
				);

				JointRef {
					refs,
					growing_radius,
					rest_angle,
					handle,
					mesh,
					flags: segment.flags,
					attachment: segment.attached_to,
				}
			})
			.collect::<Vec<_>>()
	}

	fn build_joints(world: &mut b2::World<AgentData>, joint_refs: &[JointRef]) {
		for &JointRef { handle: distal, mesh, growing_radius, rest_angle, attachment, flags, .. } in joint_refs {
			if let Some(attachment) = attachment {
				let upstream = &joint_refs[attachment.index as usize];
				let medial = upstream.handle;
				let angle_delta = rest_angle - upstream.rest_angle; //world.body(distal).angle() - world.body(medial).angle();
				let v0 = upstream.mesh.vertices[attachment.attachment_point as usize] * upstream.growing_radius;
				//let angle_delta = v0.x.atan2(-v0.y) as f32;//consts::PI;
				let v1 = mesh.vertices[0] * growing_radius;
				let a = b2::Vec2 { x: v0.x, y: v0.y };
				let b = b2::Vec2 { x: v1.x, y: v1.y };
				macro_rules! common_joint (
					($joint:ident) => {
						$joint.collide_connected = false;
						$joint.reference_angle = angle_delta;
						$joint.local_anchor_a = a;
						$joint.local_anchor_b = b;
						world.create_joint_with(&$joint, ())
					}
				);
				if flags.contains(world::segment::Flags::JOINT) {
					let mut joint = b2::RevoluteJointDef::new(medial, distal);
					joint.enable_limit = true;
					joint.upper_angle = JOINT_UPPER_ANGLE;
					joint.lower_angle = JOINT_LOWER_ANGLE;
					common_joint!(joint);
				} else {
					let mut joint = b2::WeldJointDef::new(medial, distal);
					joint.frequency = JOINT_FREQUENCY;
					joint.damping_ratio = JOINT_DAMPING_RATIO;
					common_joint!(joint);
				}
			}
		}
	}

	fn new_world(touched: ContactSet) -> b2::World<AgentData> {
		let mut world = b2::World::new(&b2::Vec2 { x: 0.0, y: -0.0 });
		world.set_contact_listener(Box::new(ContactListener { touched }));
		world
	}

	pub fn pick(&self, pos: Position) -> Option<Id> {
		let point = Self::p2v(pos);
		let eps = PICK_EPS;
		let aabb = b2::AABB {
			lower: b2::Vec2 { x: pos.x - eps, y: pos.y - eps },
			upper: b2::Vec2 { x: pos.x + eps, y: pos.y + eps },
		};
		let mut result = None;
		{
			let mut callback = |body_h: b2::BodyHandle, fixture_h: b2::FixtureHandle| {
				let body = self.world.body(body_h);
				let fixture = body.fixture(fixture_h);
				if fixture.test_point(&point) {
					result = Some(body.user_data().id());
					false
				} else {
					true
				}
			};
			self.world.query_aabb(&mut callback, &aabb);
		}
		result
	}
}

struct ContactListener {
	touched: ContactSet,
}

impl b2::ContactListener<AgentData> for ContactListener {
	fn post_solve(&mut self, ca: ContactAccess<AgentData>, _: &b2::ContactImpulse) {
		let body_a = ca.fixture_a.user_data();
		let body_b = ca.fixture_b.user_data();
		if body_a.agent_id != body_b.agent_id {
			self.touched.borrow_mut().insert(body_a.no_bone(), body_b.no_bone());
			self.touched.borrow_mut().insert(body_b.no_bone(), body_a.no_bone());
		}
	}
}
