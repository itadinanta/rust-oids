use super::*;
use core::clock::*;
use backend::world;
use core::geometry::*;
use cgmath::Vector;

pub struct GameSystem {
	hourglass: Hourglass<SystemStopwatch>,
	to_spawn: usize,
	spawned: usize,
}

impl Updateable for GameSystem {
	fn update(&mut self, _: &world::WorldState, _: f32) {
		if self.hourglass.is_expired() {
			self.hourglass.flip();
			self.to_spawn += 1;
		}
	}
}

impl System for GameSystem {
	fn from_world(&mut self, _: &world::World) {
		self.spawned = self.to_spawn;
	}
	fn to_world(&self, world: &mut world::World) {
		for i in self.spawned..self.to_spawn {
			let id = world.new_resource(Position::zero(),
			                            Some(Motion {
				                            velocity: Velocity::new((i as f32).cos(), (i as f32).sin()),
				                            spin: 0.01,
			                            }));
			world.register(id);
		}
	}
}

impl Default for GameSystem {
	fn default() -> Self {
		GameSystem {
			hourglass: Hourglass::new(0.5),
			to_spawn: 0,
			spawned: 0,
		}
	}
}

impl GameSystem {}
