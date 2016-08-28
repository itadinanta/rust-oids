use super::*;
use backend::world::WorldState;
use core::clock::Stopwatch;
use std::time::*;

pub struct GameSystem {
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

impl System for GameSystem {}

impl Default for GameSystem {
	fn default() -> Self {
		GameSystem {
			dt: 1. / 60.,
			t0: SystemTime::new(),
			frames: 0.,
			elapsed: 0.,
		}
	}
}

impl GameSystem {}
