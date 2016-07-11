use wrapped2d::b2;
use wrapped2d::user_data::*;
use rand;
use rand::Rng;
use app::obj;

use std::f64::consts;

pub trait System {
	fn update(&mut self, dt: f32);
}

pub struct CreatureData;

#[repr(packed)]
pub struct CreatureRefs {
	creature_id: obj::Id,
	limb_index: obj::LimbIndex,
	bone_index: obj::BoneIndex,
}

impl Default for CreatureRefs {
	fn default() -> CreatureRefs {
		CreatureRefs {
			creature_id: 0xdeadbeef,
			limb_index: 0xff,
			bone_index: 0xff,
		}
	}
}

impl UserDataTypes for CreatureData {
	type BodyData = CreatureRefs;
    type JointData = ();
    type FixtureData = CreatureRefs;
}

pub struct PhysicsSystem {
	edge: f32,
	world: b2::World<CreatureData>,
}

impl System for PhysicsSystem {
	fn update(&mut self, dt: f32) {
		let world = &mut self.world;
		world.step(dt, 8, 3);
		const MAX_RADIUS: f32 = 5.0;
		let mut v = Vec::new();
		for (h, b) in world.bodies() {
			let body = b.borrow();
			let position = (*body).position();
			if position.y < (self.edge - MAX_RADIUS) {
				v.push(h);
			}
		}
		for h in v {
			world.destroy_body(h);
		}
	}
}

impl PhysicsSystem {
	pub fn new() -> Self {
		PhysicsSystem {
			world: Self::new_world(),
			edge: 0.,
		}
	}

	pub fn drop_below(&mut self, edge: f32) {
		self.edge = edge;
	}

	pub fn new_ball(&mut self, pos: obj::Position) {
		let mut world = &self.world;
		let mut rng = rand::thread_rng();
		let radius: f32 = (rng.gen::<f32>() * 1.0) + 1.0;

		let mut circle_shape = b2::CircleShape::new();
		circle_shape.set_radius(radius);

		let mut f_def = b2::FixtureDef::new();
		f_def.density = (rng.gen::<f32>() * 1.0) + 1.0;
		f_def.restitution = 0.2;
		f_def.friction = 0.3;

		let mut b_def = b2::BodyDef::new();
		b_def.body_type = b2::BodyType::Dynamic;
		b_def.position = b2::Vec2 {
			x: pos.x,
			y: pos.y,
		};
		let handle = world.create_body(&b_def);
		world.body_mut(handle)
		     .create_fixture(&circle_shape, &mut f_def);
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

pub struct GameSystem {
	players: obj::Flock,
	friends: obj::Flock,
	enemies: obj::Flock,
	crowds: obj::Flock,
}

impl GameSystem {
	pub fn new() -> GameSystem {
		GameSystem {
			players: obj::Flock::new(),
			friends: obj::Flock::new(),
			enemies: obj::Flock::new(),
			crowds: obj::Flock::new(),
		}
	}
}
