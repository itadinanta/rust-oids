use super::*;
use backend::world;
use backend::world::WorldState;
use core::clock::Stopwatch;
use std::time::*;
use cgmath::*;
use core::geometry::Position;

pub struct AlifeSystem {
	t0: SystemTime,
	dt: f32,
	frames: f32,
	elapsed: f32,
	source: Position,
}

impl Updateable for AlifeSystem {
	fn update(&mut self, _: &WorldState, dt: f32) {
		self.dt = dt;
		self.frames += dt;
		self.elapsed = self.t0.seconds();
	}
}

impl System for AlifeSystem {
	fn to_world(&self, world: &mut world::World) {
		for (_, agent) in world.minions.agents_mut().iter_mut() {
			if agent.state.is_active() {
				if agent.state.lifespan().is_expired() {
					agent.state.die();
				} else {
					for segment in agent.segments_mut() {
						segment.state.update(self.dt);
					}
				}
			}
		}
	}
}

impl Default for AlifeSystem {
	fn default() -> Self {
		AlifeSystem {
			dt: 1. / 60.,
			t0: SystemTime::new(),
			frames: 0.,
			elapsed: 0.,
			source: Position::zero(),
		}
	}
}

impl AlifeSystem {
	pub fn source(&mut self, pos: Position) {
		self.source = pos;
	}
}
