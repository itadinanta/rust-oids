use super::*;
use backend::world;
use backend::world::WorldState;
use core::clock::Stopwatch;
use std::time::*;

pub struct GameSystem {
	speed: f32,
	t0: SystemTime,
	dt: f32,
	frames: f32,
	elapsed: f32,
}

impl Updateable for GameSystem {
	fn update(&mut self, _: &WorldState, dt: f32) {
		self.dt = dt;
		self.frames += dt;
		self.elapsed = self.t0.seconds();
	}
}

impl System for GameSystem {
	fn to_world(&self, world: &mut world::World) {
		let keys: Vec<_> = world.minions.agents().keys().cloned().collect();
		for k in keys {
			if let Some(b) = world.minions.get_mut(k) {
				for segment in b.segments_mut() {
					segment.state.update(self.dt * self.speed);
				}
			}
		}
	}
}

impl GameSystem {
	pub fn new() -> Self {
		GameSystem {
			dt: 1. / 60.,
			speed: 1.,
			t0: SystemTime::new(),
			frames: 0.,
			elapsed: 0.,
		}
	}
}
