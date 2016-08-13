use wrapped2d::b2;
use wrapped2d::user_data::*;
use backend::obj;
use super::*;
use backend::obj::{Solid, Geometry, Transformable};
use backend::world;
use std::collections::HashMap;
use std::f32::consts;

struct AgentData;

impl UserDataTypes for AgentData {
	type BodyData = world::AgentRefs;
	type JointData = ();
	type FixtureData = world::AgentRefs;
}

pub struct PhysicsSystem {
	edge: f32,
	remote: obj::Position,
	world: b2::World<AgentData>,
	handles: HashMap<world::AgentRefs, b2::BodyHandle>,
	dropped: Vec<world::AgentRefs>,
}

use cgmath::Vector;
use cgmath::EuclideanVector;

impl Updateable for PhysicsSystem {
	fn update(&mut self, state: &world::WorldState, dt: f32) {
		enum BodyForce {
			Parallel(b2::BodyHandle, f32, b2::Vec2, b2::Vec2),
			Perpendicular(b2::BodyHandle, f32, b2::Vec2, b2::Vec2),
		}
		let mut v = Vec::new();

		for (h, b) in self.world.bodies() {
			let body = b.borrow();
			let center = (*body).world_center().clone();
			let facing = (*body).world_point(&b2::Vec2 { x: 0., y: 1. }).clone();
			let key = (*body).user_data();
			if let Some(segment) = state.minion(key.agent_id).and_then(|c| c.segment(key.segment_index)) {
				let power = segment.state.charge * segment.mesh.shape.radius().powi(2);
				if segment.flags.contains(world::RUDDER) {
					v.push(BodyForce::Perpendicular(h, power, center, facing));
				}
				if segment.flags.contains(world::THRUSTER) {
					v.push(BodyForce::Parallel(h, power, center, facing));
				}
			}
		}
		for force in v {
			match force {
				BodyForce::Perpendicular(h, power, center, facing) => {
					let t = self.remote - obj::Position::new(center.x, center.y);
					let f = obj::Position::new(center.x - facing.x, center.y - facing.y);
					let d = f.dot(t);

					if d > 0. {
						let b = &mut self.world.body_mut(h);
						let power = power * 10.;
						let f = f.normalize_to(power);
						b.apply_force(&b2::Vec2 { x: f.x, y: f.y }, &center, true);
					}
				}
				BodyForce::Parallel(h, power, center, facing) => {
					let t = self.remote - obj::Position::new(center.x, center.y);
					let f = obj::Position::new(facing.x - center.x, facing.y - center.y);
					// let d = f.dot(t);

					//if d > 0. {
						let power = power * 50.;
						let f = f.normalize_to(power);
						self.world.body_mut(h).apply_force(&b2::Vec2 { x: f.x, y: f.y }, &center, true);
					//} else {
					//	let power = power * 10.;
					//	let f = f.normalize_to(power);
					//	self.world.body_mut(h).apply_force(&b2::Vec2 { x: -f.x, y: -f.y }, &center, true);
					//}
				}
			}
		}

		self.world.step(dt, 8, 3);
	}
}

struct JointRef<'a> {
	refs: world::AgentRefs,
	handle: b2::BodyHandle,
	mesh: &'a obj::Mesh,
	flags: world::SegmentFlags,
	attachment: Option<world::Attachment>,
}

impl System for PhysicsSystem {
	fn register(&mut self, agent: &world::Agent) {
		// build fixtures
		let joint_refs = PhysicsSystem::build_fixtures(&mut self.world, &agent);
		// and then assemble them with joints
		PhysicsSystem::build_joints(&mut self.world, &joint_refs);
		// record them
		for JointRef { refs, handle, .. } in joint_refs {
			self.handles.insert(refs, handle);
		}
	}

	fn from_world(&self, _: &world::World) {}

	fn to_world(&self, world: &mut world::World) {
		for key in &self.dropped {
			world.minions.kill(&key.agent_id);
			// println!("Killed object: {}", key.agent_id);
		}
		for (_, b) in self.world.bodies() {
			let body = b.borrow();
			let position = (*body).position();
			let angle = (*body).angle();
			let key = (*body).user_data();

			if let Some(agent) = world.minions.get_mut(key.agent_id) {
				if let Some(object) = agent.segment_mut(key.segment_index) {
					let scale = object.transform().scale;
					object.transform_to(obj::Transform {
						position: obj::Position {
							x: position.x,
							y: position.y,
						},
						angle: angle,
						scale: scale,
					});
				}
			}
		}
	}
}

impl PhysicsSystem {
	pub fn new() -> Self {
		PhysicsSystem {
			world: Self::new_world(),
			edge: 0.,
			remote: obj::Position::new(0., 0.),
			handles: HashMap::new(),
			dropped: Vec::new(),
		}
	}

	fn build_fixtures<'a>(world: &mut b2::World<AgentData>, agent: &'a world::Agent) -> Vec<JointRef<'a>> {
		let object_id = agent.id();
		let segments = agent.segments();
		segments.enumerate()
			.map(|(segment_index, segment)| {
				let material = segment.material();
				let mut f_def = b2::FixtureDef::new();
				f_def.density = material.density;
				f_def.restitution = material.restitution;
				f_def.friction = material.friction;

				let transform = segment.transform();
				let mut b_def = b2::BodyDef::new();
				b_def.body_type = b2::BodyType::Dynamic;
				b_def.linear_damping = 0.5;
				b_def.angular_damping = 0.8;
				b_def.angle = transform.angle;
				b_def.position = b2::Vec2 {
					x: transform.position.x,
					y: transform.position.y,
				};
				let refs = world::AgentRefs::with_segment(object_id, segment_index as u8);
				let handle = world.create_body_with(&b_def, refs);

				let mesh = segment.mesh();
				let flags = segment.flags;
				let attached_to = segment.attached_to;
				match mesh.shape {
					obj::Shape::Ball { radius } => {
						let mut circle_shape = b2::CircleShape::new();
						circle_shape.set_radius(radius);
						world.body_mut(handle).create_fixture_with(&circle_shape, &mut f_def, refs);
					}
					obj::Shape::Box { radius, ratio } => {
						let mut rect_shape = b2::PolygonShape::new();
						rect_shape.set_as_box(radius * ratio, radius);
						world.body_mut(handle).create_fixture_with(&rect_shape, &mut f_def, refs);
					}
					obj::Shape::Star { radius, n, .. } => {
						let p = &mesh.vertices;
						for i in 0..n {
							let mut quad = b2::PolygonShape::new();
							let i1 = (i * 2 + 1) as usize;
							let i2 = (i * 2) as usize;
							let i3 = ((i * 2 + (n * 2) - 1) % (n * 2)) as usize;
							let (p1, p2, p3) = match mesh.winding {
								obj::Winding::CW => (&p[i1], &p[i2], &p[i3]),
								obj::Winding::CCW => (&p[i1], &p[i3], &p[i2]),
							};
							quad.set(&[b2::Vec2 { x: 0., y: 0. },
							           b2::Vec2 {
								           x: p1.x * radius,
								           y: p1.y * radius,
							           },
							           b2::Vec2 {
								           x: p2.x * radius,
								           y: p2.y * radius,
							           },
							           b2::Vec2 {
								           x: p3.x * radius,
								           y: p3.y * radius,
							           }]);
							let refs = world::AgentRefs::with_bone(object_id, segment_index as u8, i as u8);
							world.body_mut(handle).create_fixture_with(&quad, &mut f_def, refs);
						}
					}
					obj::Shape::Triangle { radius, .. } => {
						let p = &mesh.vertices;
						let mut tri = b2::PolygonShape::new();
						let (p1, p2, p3) = match mesh.winding {
							obj::Winding::CW => (&p[0], &p[2], &p[1]),
							obj::Winding::CCW => (&p[0], &p[1], &p[2]),
						};
						tri.set(&[b2::Vec2 {
							          x: p1.x * radius,
							          y: p1.y * radius,
						          },
						          b2::Vec2 {
							          x: p2.x * radius,
							          y: p2.y * radius,
						          },
						          b2::Vec2 {
							          x: p3.x * radius,
							          y: p3.y * radius,
						          }]);
						world.body_mut(handle).create_fixture_with(&tri, &mut f_def, refs);
					}
				};
				JointRef {
					refs: refs,
					handle: handle,
					mesh: mesh,
					flags: flags,
					attachment: attached_to,
				}
			})
			.collect::<Vec<_>>()
	}

	fn build_joints(world: &mut b2::World<AgentData>, joint_refs: &Vec<JointRef>) {
		for &JointRef { handle: distal, mesh, attachment, flags, .. } in joint_refs {
			if let Some(attachment) = attachment {
				let upstream = &joint_refs[attachment.index as usize];
				let medial = upstream.handle;
				let angle_delta = world.body(distal).angle() - world.body(medial).angle();

				let v0 = upstream.mesh.vertices[attachment.attachment_point as usize] * upstream.mesh.shape.radius();
				let v1 = mesh.vertices[0] * mesh.shape.radius();
				let a = b2::Vec2 { x: v0.x, y: v0.y };
				let b = b2::Vec2 { x: v1.x, y: v1.y };
				if flags.contains(world::JOINT) {
					let mut joint = b2::RevoluteJointDef::new(medial, distal);
					joint.collide_connected = false;
					joint.reference_angle = angle_delta;
					joint.enable_limit = true;
					joint.upper_angle = consts::PI / 6.;
					joint.lower_angle = -consts::PI / 6.;
					joint.local_anchor_a = a;
					joint.local_anchor_b = b;
					world.create_joint_with(&joint, ());
				} else {
					// TODO: how do we reduce the clutter?
					let mut joint = b2::WeldJointDef::new(medial, distal);
					joint.collide_connected = false;
					joint.reference_angle = angle_delta;
					joint.frequency = 5.0;
					joint.damping_ratio = 0.9;
					joint.local_anchor_a = a;
					joint.local_anchor_b = b;
					world.create_joint_with(&joint, ());
				}
			}
		}
	}

	pub fn drop_below(&mut self, edge: f32) {
		self.edge = edge;
	}

	pub fn follow_me(&mut self, pos: obj::Position) {
		self.remote = pos;
	}

	fn new_world() -> b2::World<AgentData> {
		let world = b2::World::new(&b2::Vec2 { x: 0.0, y: 0.0 });

		world
	}
}
