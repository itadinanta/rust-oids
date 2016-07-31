use wrapped2d::b2;
use wrapped2d::user_data::*;
use backend::obj;
use backend::obj::Updateable;
use super::*;
use backend::obj::{Solid, Geometry, Transformable};
use backend::world;
use std::collections::HashMap;
use std::f64::consts;

struct CreatureData;

impl UserDataTypes for CreatureData {
	type BodyData = world::CreatureRefs;
    type JointData = ();
    type FixtureData = world::CreatureRefs;
}

pub struct PhysicsSystem {
	edge: f32,
	world: b2::World<CreatureData>,
	handles: HashMap<world::CreatureRefs, b2::BodyHandle>,
	dropped: Vec<world::CreatureRefs>,
}

impl Updateable for PhysicsSystem {
	fn update(&mut self, dt: f32) {
		let world = &mut self.world;
		world.step(dt, 8, 3);
		const MAX_RADIUS: f32 = 5.0;
		let mut v = Vec::new();
		self.dropped.clear();
		// TODO: is this the best way to iterate?
		for (h, b) in world.bodies() {
			let body = b.borrow();
			let position = (*body).position();
			let key = (*body).user_data().clone();
			if position.y < (self.edge - MAX_RADIUS) {
				v.push(h);
				self.dropped.push(key);
			}
		}
		for h in v {
			world.destroy_body(h);
		}
		for key in &self.dropped {
			self.handles.remove(&key);
		}
	}
}

impl System for PhysicsSystem {
	fn register(&mut self, creature: &world::Creature) {
		let world = &mut self.world;
		let object_id = creature.id();

		let mut joint_body: Option<b2::BodyHandle> = None;
		let mut joint_limb: Option<b2::BodyHandle> = None;

		for (limb_index, limb) in creature.limbs().enumerate() {
			let material = limb.material();
			let mut f_def = b2::FixtureDef::new();
			f_def.density = material.density;
			f_def.restitution = material.restitution;
			f_def.friction = material.friction;

			let transform = limb.transform();
			let mut b_def = b2::BodyDef::new();
			b_def.body_type = b2::BodyType::Dynamic;
			b_def.position = b2::Vec2 {
				x: transform.position.x,
				y: transform.position.y,
			};
			let refs = world::CreatureRefs::with_limb(object_id, limb_index as u8);
			let handle = world.create_body_with(&b_def, refs);

			let mesh = limb.mesh();

			match mesh.shape {
				obj::Shape::Ball { radius } => {
					let mut circle_shape = b2::CircleShape::new();
					circle_shape.set_radius(radius);
					world.body_mut(handle).create_fixture_with(&circle_shape, &mut f_def, refs);
				}
				obj::Shape::Box { width, height } => {
					let mut rect_shape = b2::PolygonShape::new();
					rect_shape.set_as_box(width, height);
					world.body_mut(handle).create_fixture_with(&rect_shape, &mut f_def, refs);
				}
				obj::Shape::Star { radius, n, .. } => {
					let p = &mesh.vertices;
					for i in 0..n {
						let mut quad = b2::PolygonShape::new();
						let p1 = p[(i * 2 + 1) as usize];
						let p2 = p[(i * 2) as usize];
						let p3 = p[((i * 2 + (n * 2) - 1) % (n * 2)) as usize];
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
						let refs = world::CreatureRefs::with_bone(object_id, limb_index as u8, i as u8);
						world.body_mut(handle).create_fixture_with(&quad, &mut f_def, refs);
					}
				}
				obj::Shape::Triangle { radius, .. } => {
					let p = &mesh.vertices;
					let mut quad = b2::PolygonShape::new();
					let p1 = p[0];
					let p2 = p[2]; // not a typo, we want cw, physics wants ccw
					let p3 = p[1];
					quad.set(&[b2::Vec2 {
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
					world.body_mut(handle).create_fixture_with(&quad, &mut f_def, refs);
				}
			};
			if joint_body == None {
				joint_body = Some(handle);
			} else {
				joint_limb = Some(handle);
			}
			if let (Some(b), Some(l)) = (joint_body, joint_limb) {
				let mut joint = b2::RevoluteJointDef::new(b, l);
				joint.reference_angle = 1.0;
				joint.local_anchor_b = b2::Vec2 { x: 0., y: 2. };
				world.create_joint_with(&joint, ());
			}
			self.handles.insert(refs, handle);
		}
	}

	fn to_world(&self, world: &mut world::World) {
		for key in &self.dropped {
			world.friends.kill(&key.creature_id);
			// println!("Killed object: {}", key.creature_id);
		}
		for (_, b) in self.world.bodies() {
			let body = b.borrow();
			let position = (*body).position();
			let angle = (*body).angle();
			let key = (*body).user_data();

			if let Some(creature) = world.friends.get_mut(key.creature_id) {
				if let Some(object) = creature.limb_mut(key.limb_index) {
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
			handles: HashMap::new(),
			dropped: Vec::new(),
		}
	}

	pub fn drop_below(&mut self, edge: f32) {
		self.edge = edge;
	}

	fn new_world() -> b2::World<CreatureData> {
		let mut world = b2::World::new(&b2::Vec2 { x: 0.0, y: -9.8 });

		let mut b_def = b2::BodyDef::new();
		b_def.body_type = b2::BodyType::Static;
		b_def.position = b2::Vec2 { x: 0.0, y: -8.0 };

		let mut ground_box = b2::PolygonShape::new();
		{
			ground_box.set_as_box(20.0, 1.0);
			let ground_handle = world.create_body(&b_def);
			let ground = &mut world.body_mut(ground_handle);
			ground.create_fast_fixture(&ground_box, 0.);

			ground_box.set_as_oriented_box(1.0,
			                               5.0,
			                               &b2::Vec2 { x: 21.0, y: 5.0 },
			                               (-consts::FRAC_PI_8) as f32);
			ground.create_fast_fixture(&ground_box, 0.);

			ground_box.set_as_oriented_box(1.0,
			                               5.0,
			                               &b2::Vec2 { x: -21.0, y: 5.0 },
			                               (consts::FRAC_PI_8) as f32);
			ground.create_fast_fixture(&ground_box, 0.);
		}
		world
	}
}
