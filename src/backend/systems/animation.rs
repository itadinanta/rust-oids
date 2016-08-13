use super::*;
use backend::world;
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
		self.dt = dt;
		self.frames += dt;
		if let Ok(dt) = self.t0.elapsed() {
			self.elapsed = (dt.as_secs() as f32) + (dt.subsec_nanos() as f32) * 1e-9;
		};
	}
}

impl System for AnimationSystem {
	fn register(&mut self, _: &world::Agent) {}

	fn from_world(&self, world: &world::World) {}

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

impl AnimationSystem {
	pub fn new() -> Self {
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
