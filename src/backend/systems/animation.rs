use super::*;
use backend::world::WorldState;
use std::time::*;

pub struct AnimationSystem {
	speed: f32,
	t0: SystemTime,
	now: SystemTime,
	dt: f32,
	frames: f32,
	elapsed: f32,
}

impl Updateable for AnimationSystem {
	fn update(&mut self, _: &WorldState, dt: f32) {
		self.now = SystemTime::now();
		self.dt = dt * self.speed;
		self.frames += self.dt;
		if let Ok(dt) = self.t0.elapsed() {
			self.elapsed = (dt.as_secs() as f32) + (dt.subsec_nanos() as f32) * 1e-9;
		};
	}
}

impl System for AnimationSystem {}

impl Default for AnimationSystem {
	fn default() -> Self {
		AnimationSystem {
			dt: 1. / 60.,
			speed: 1.,
			t0: SystemTime::now(),
			now: SystemTime::now(),
			frames: 0.,
			elapsed: 0.,
		}
	}
}

impl AnimationSystem {}
