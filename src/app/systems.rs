use wrapped2d::b2;
use wrapped2d::user_data::*;
use app::obj;
use app::obj::Transformable;
use app::world;
use std::collections::HashMap;
use std::f64::consts;

pub trait System {
	fn update(&mut self, dt: f32);
	fn register(&mut self, creature: &world::Creature);
	fn update_world(&self, world: &mut world::World);
}

pub struct CreatureData;

impl UserDataTypes for CreatureData {
	type BodyData = world::CreatureRefs;
    type JointData = ();
    type FixtureData = world::CreatureRefs;
}

pub struct PhysicsSystem {
	edge: f32,
	world: b2::World<CreatureData>,
	handles: HashMap<world::CreatureRefs, b2::BodyHandle>,
}

impl System for PhysicsSystem {
	fn update(&mut self, dt: f32) {
		let world = &mut self.world;
		world.step(dt, 8, 3);
		const MAX_RADIUS: f32 = 5.0;
		let mut v = Vec::new();
		let mut keys = Vec::new();
		// TODO: is this the best way to iterate?
		for (h, b) in world.bodies() {
			let body = b.borrow();
			let position = (*body).position();
			let key = (*body).user_data().clone();
			if position.y < (self.edge - MAX_RADIUS) {
				v.push(h);
				keys.push(key);
			}
		}
		for h in v {
			world.destroy_body(h);
		}
		for key in keys {
			self.handles.remove(&key);
		}
	}

	fn register(&mut self, creature: &world::Creature) {
		let world = &mut self.world;
		let object_id = creature.id();
		for (limb_index, limb) in creature.limbs().enumerate() {
			let shape = match limb.mesh.shape {
				obj::Shape::Ball { radius } => {
					let mut circle_shape = b2::CircleShape::new();
					circle_shape.set_radius(radius);
					Some(circle_shape)
				}
				_ => None,
			};
			let mut f_def = b2::FixtureDef::new();
			f_def.density = limb.material.density;
			f_def.restitution = limb.material.restitution;
			f_def.friction = limb.material.friction;

			let mut b_def = b2::BodyDef::new();
			b_def.body_type = b2::BodyType::Dynamic;
			b_def.position = b2::Vec2 {
				x: limb.transform.position.x,
				y: limb.transform.position.y,
			};
			let refs = world::CreatureRefs::with_limb(object_id, limb_index as u8);
			let handle = world.create_body_with(&b_def, refs);
			world.body_mut(handle).create_fixture_with(&shape.unwrap(), &mut f_def, refs);
			self.handles.insert(refs, handle);
		}
	}

	fn update_world(&self, world: &mut world::World) {
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
