use super::*;
use backend::world::WorldState;
use core::clock::*;

pub struct AnimationSystem {
	speed: f32,
	t0: SystemStopwatch,
	now: SystemStopwatch,
	dt: f32,
	frames: f32,
	elapsed: f32,
}

impl Updateable for AnimationSystem {
	fn update(&mut self, _: &WorldState, dt: f32) {
		self.now.reset();
		self.dt = dt * self.speed;
		self.frames += self.dt;
		self.elapsed = self.t0.seconds();
	}
}

impl System for AnimationSystem {}

impl Default for AnimationSystem {
	fn default() -> Self {
		AnimationSystem {
			dt: 1. / 60.,
			speed: 1.,
			t0: Stopwatch::new(),
			now: Stopwatch::new(),
			frames: 0.,
			elapsed: 0.,
		}
	}
}

impl AnimationSystem {}
