use super::*;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use app::constants::*;
use wrapped2d::b2;
use wrapped2d::user_data::*;
use wrapped2d::dynamics::world::callbacks::ContactAccess;
use core::geometry::*;
use core::geometry::Transform;
use cgmath::InnerSpace;
use backend::obj;
use backend::obj::*;
use backend::world;
use backend::world::agent;
use backend::world::segment;
use backend::world::segment::Intent;
use backend::world::segment::PilotRotation;

struct AgentData;

impl UserDataTypes for AgentData {
	type BodyData = agent::Key;
	type JointData = ();
	type FixtureData = agent::Key;
}

type ContactSet = Rc<RefCell<HashMap<agent::Key, agent::Key>>>;

pub struct PhysicsSystem {
	world: b2::World<AgentData>,
	handles: HashMap<agent::Key, b2::BodyHandle>,
	touched: ContactSet,
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
	mesh: &'a obj::Mesh,
	flags: segment::Flags,
	attachment: Option<segment::Attachment>,
}

impl System for PhysicsSystem {
	fn init(&mut self, world: &world::World) {
		self.init_extent(&world.extent);
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

	fn update(&mut self, state: &world::AgentState, dt_sec: Seconds) {
		use self::BodyUpdate::*;
		let mut body_updates = Vec::new();
		let dt: f32 = dt_sec.into();
		#[inline]
		fn to_vec2(v: Position) -> b2::Vec2 { PhysicsSystem::to_vec2(&v) };
		#[inline]
		fn from_vec2(v: &b2::Vec2) -> Position { PhysicsSystem::from_vec2(v) };
		for (h, b) in self.world.bodies() {
			let body = b.borrow();
			let center = (*body).world_center().clone();
			let key = (*body).user_data();
			if let Some(segment) = state.agent(key.agent_id)
				.and_then(|c| c.segment(key.segment_index)) {
				match segment.state.intent {
					Intent::Move(force) =>
						body_updates.push((h, Force(center, to_vec2(force)))),
					Intent::Brake(force) => {
						let linear_velocity = from_vec2((*body).linear_velocity());
						let comp = force.dot(linear_velocity);
						if comp < 0. {
							body_updates.push((h, Force(center, to_vec2(force))));
						}
					}
					Intent::PilotTo(force, ref target_angle) => {
						if let Some(force) = force {
							let linear_velocity = from_vec2((*body).linear_velocity());
							let speed = linear_velocity.magnitude2();
							let drag_factor = (1. - (speed * speed) * DRAG_COEFFICIENT).min(1.).max(0.);
							body_updates.push((h, Force(center, to_vec2(force * drag_factor))));
						}
						match target_angle {
							&PilotRotation::LookAt(target) => {
								let look_at_vector = target - from_vec2(&center);
								let target_angle = f32::atan2(-look_at_vector.x, look_at_vector.y);
								body_updates.push((h, Transform(*(*body).position(), target_angle)));
							}
							&PilotRotation::Orientation(direction) => {
								let target_angle = f32::atan2(-direction.x, direction.y);
								body_updates.push((h, Transform(*(*body).position(), target_angle)));
							}
							&PilotRotation::Turn(angle) => {
								body_updates.push((h, Torque(angle)));
							}
							&PilotRotation::None => {}
							//TODO: try physics!
							//let angle = (*body).angle();
							//let norm_diff = math::normalize_rad(target_angle - angle);
							//body_updates.push((h, Torque(norm_diff * COMPASS_SPRING_POWER)))
							//torques.push((h, norm_diff * COMPASS_SPRING_POWER));
						}
					}
					Intent::RunAway(impulse) =>
						body_updates.push((h, LinearImpulse(center, to_vec2(impulse * dt)))),
					_ => {}
				}
			}
		}

		for (h, update) in body_updates {
			let b = &mut self.world.body_mut(h);
			match update {
				BodyUpdate::Torque(torque) =>
					b.apply_torque(torque, true),
				BodyUpdate::AngularImpulse(impulse) =>
					b.apply_angular_impulse(impulse, true),
				BodyUpdate::Force(application_point, force) =>
					b.apply_force(&force, &application_point, true),
				BodyUpdate::LinearImpulse(application_point, impulse) =>
					b.apply_linear_impulse(&impulse, &application_point, true),
				BodyUpdate::Transform(translation, rotation) =>
					b.set_transform(&translation, rotation),
			}
		}
		self.world.step(dt, 8, 3);
	}

	fn export(&self, world: &mut world::World) {
		for (_, b) in self.world.bodies() {
			let body = b.borrow();
			let position = (*body).position();
			let angle = (*body).angle();
			let key = (*body).user_data();

			if let Some(agent) = world.agent_mut(key.agent_id) {
				if let Some(segment) = agent.segment_mut(key.segment_index) {
					segment.transform_to(&Transform::new(PhysicsSystem::from_vec2(&position), angle));
					segment.state.last_touched = self.touched.borrow().get(key).map(|r| *r);
				}
			}
		}
		self.touched.borrow_mut().clear();
	}
}

impl Default for PhysicsSystem {
	fn default() -> Self {
		let touched = Rc::new(RefCell::new(HashMap::new()));
		PhysicsSystem {
			world: Self::new_world(touched.clone()),
			handles: HashMap::new(),
			touched,
		}
	}
}

impl PhysicsSystem {
	fn to_vec2(p: &Position) -> b2::Vec2 {
		b2::Vec2 { x: p.x, y: p.y }
	}

	fn from_vec2(p: &b2::Vec2) -> Position {
		Position::new(p.x, p.y)
	}

	fn init_extent(&mut self, extent: &Rect) {
		let mut f_def = b2::FixtureDef::new();
		let mut b_def = b2::BodyDef::new();
		b_def.body_type = b2::BodyType::Static;
		let refs = agent::Key::with_id(0xFFFFFFFFusize);
		let handle = self.world.create_body_with(&b_def, refs);

		let mut rect = b2::ChainShape::new();
		rect.create_loop(
			&[
				Self::to_vec2(&extent.bottom_left()),
				Self::to_vec2(&extent.bottom_right()),
				Self::to_vec2(&extent.top_right()),
				Self::to_vec2(&extent.top_left()),
			],
		);

		self.world.body_mut(handle).create_fixture_with(
			&rect,
			&mut f_def,
			refs,
		);
	}

	fn vec2(p: &Position, radius: f32) -> b2::Vec2 {
		b2::Vec2 {
			x: p.x * radius,
			y: p.y * radius,
		}
	}

	fn build_fixture_for_segment(world: &mut b2::World<AgentData>,
								 handle: b2::BodyHandle,
								 object_id: obj::Id,
								 segment_index: usize,
								 refs: agent::Key,
								 f_def: &mut b2::FixtureDef,
								 mesh: &Mesh) {
		match mesh.shape {
			obj::Shape::Ball { radius } => {
				let mut circle_shape = b2::CircleShape::new();
				circle_shape.set_radius(radius);
				world.body_mut(handle).create_fixture_with(&circle_shape, f_def, refs);
			}
			obj::Shape::Box { radius, ratio } => {
				let mut rect_shape = b2::PolygonShape::new();
				rect_shape.set_as_box(radius * ratio, radius);
				world.body_mut(handle).create_fixture_with(&rect_shape, f_def, refs);
			}
			obj::Shape::Poly { radius, n, .. } => {
				let p = &mesh.vertices;
				let offset = if n < 0 { 1 } else { 0 };
				let mut poly = b2::PolygonShape::new();
				let mut vertices = Vec::new();
				for i in 0..n.abs() {
					vertices.push(Self::vec2(&p[2 * i as usize + offset], radius));
				}
				poly.set(vertices.as_slice());
				world.body_mut(handle).create_fixture_with(&poly, f_def, refs);
			}
			obj::Shape::Star { radius, n, .. } => {
				let p = &mesh.vertices;
				for i in 0..n {
					let mut quad = b2::PolygonShape::new();
					let i1 = (i * 2 + 1) as usize;
					let i2 = (i * 2) as usize;
					let i3 = ((i * 2 + (n * 2) - 1) % (n * 2)) as usize;
					let (p1, p2, p3) = match mesh.winding() {
						obj::Winding::CW => (&p[i1], &p[i2], &p[i3]),
						obj::Winding::CCW => (&p[i1], &p[i3], &p[i2]),
					};
					quad.set(
						&[
							b2::Vec2 { x: 0., y: 0. },
							Self::vec2(&p1, radius),
							Self::vec2(&p2, radius),
							Self::vec2(&p3, radius),
						],
					);
					let refs = agent::Key::with_bone(object_id, segment_index as u8, i as u8);
					world.body_mut(handle).create_fixture_with(&quad, f_def, refs);
				}
			}
			obj::Shape::Triangle { radius, .. } => {
				let p = &mesh.vertices;
				let mut tri = b2::PolygonShape::new();
				let (p1, p2, p3) = match mesh.winding() {
					obj::Winding::CW => (&p[0], &p[2], &p[1]),
					obj::Winding::CCW => (&p[0], &p[1], &p[2]),
				};
				tri.set(
					&[
						Self::vec2(p1, radius),
						Self::vec2(p2, radius),
						Self::vec2(p3, radius),
					],
				);
				world.body_mut(handle).create_fixture_with(&tri, f_def, refs);
			}
		};
	}

	fn build_fixtures<'a>(world: &mut b2::World<AgentData>, agent: &'a world::agent::Agent) -> Vec<JointRef<'a>> {
		let object_id = agent.id();
		let segments = agent.segments();
		segments
			.into_iter()
			.enumerate()
			.map(|(segment_index, segment)| {
				let material = segment.material();
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
				b_def.position = Self::vec2(&transform.position, 1.);
				if let Some(Motion { velocity, spin }) = segment.motion {
					b_def.linear_velocity = Self::vec2(&velocity, 1.);
					b_def.angular_velocity = spin;
				}
				let refs = agent::Key::with_segment(object_id, segment_index as u8);
				let handle = world.create_body_with(&b_def, refs);
				let mesh = segment.mesh();

				Self::build_fixture_for_segment(world, handle, object_id, segment_index, refs, &mut f_def, mesh);

				JointRef {
					refs,
					handle,
					mesh,
					flags: segment.flags,
					attachment: segment.attached_to,
				}
			})
			.collect::<Vec<_>>()
	}

	fn build_joints(world: &mut b2::World<AgentData>, joint_refs: &Vec<JointRef>) {
		for &JointRef {
			handle: distal,
			mesh,
			attachment,
			flags,
			..
		} in joint_refs
			{
				if let Some(attachment) = attachment {
					let upstream = &joint_refs[attachment.index as usize];
					let medial = upstream.handle;
					let angle_delta = world.body(distal).angle() - world.body(medial).angle();

					let v0 = upstream.mesh.vertices[attachment.attachment_point as usize] * upstream.mesh.shape.radius();
					let v1 = mesh.vertices[0] * mesh.shape.radius();
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
		let point = Self::to_vec2(&pos);
		let eps = PICK_EPS;
		let aabb = b2::AABB {
			lower: b2::Vec2 {
				x: pos.x - eps,
				y: pos.y - eps,
			},
			upper: b2::Vec2 {
				x: pos.x + eps,
				y: pos.y + eps,
			},
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
			self.touched.borrow_mut().insert(
				body_a.no_bone(),
				body_b.no_bone(),
			);
			self.touched.borrow_mut().insert(
				body_b.no_bone(),
				body_a.no_bone(),
			);
		}
	}
}
