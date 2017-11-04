use super::*;
use backend::world::WorldState;
use core::clock::*;

pub struct AnimationSystem<T: Stopwatch> {
	speed: f32,
	t0: T,
	now: T,
	dt: f32,
	frames: f32,
	elapsed: f32,
}

impl<T> Updateable for AnimationSystem<T> where T: Stopwatch {
	fn update(&mut self, _: &WorldState, dt: f32) {
		self.now.reset();
		self.dt = dt * self.speed;
		self.frames += self.dt;
		self.elapsed = self.t0.seconds();
		self.now.tick(dt);
		self.t0.tick(dt);
	}
}

impl<T> System for AnimationSystem<T> where T: Stopwatch {}

impl<T> Default for AnimationSystem<T> where T: Stopwatch {
	fn default() -> Self {
		AnimationSystem {
			dt: 1. / 60.,
			speed: 1.,
			t0: T::new(),
			now: T::new(),
			frames: 0.,
			elapsed: 0.,
		}
	}
}

impl<T> AnimationSystem<T> where T: Stopwatch {}
