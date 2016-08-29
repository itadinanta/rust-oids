use super::*;
use backend::world;
use backend::world::segment;
use backend::world::WorldState;
use cgmath::*;
use core::geometry::Position;

pub struct AlifeSystem {
	dt: f32,
	source: Position,
}

impl Updateable for AlifeSystem {
	fn update(&mut self, _: &WorldState, dt: f32) {
		self.dt = dt;
	}
}

impl System for AlifeSystem {
	fn to_world(&self, world: &mut world::World) {
		for (_, agent) in world.minions.agents_mut().iter_mut() {
			if agent.state.is_active() {
				if agent.state.power() > 0. {
					agent.state.renew();
				}
				if agent.state.lifespan().is_expired() {
					agent.state.die();
				} else {
					for segment in agent.segments.iter_mut() {
						// some source of food, let's use the light
						let d = (self.source - segment.transform.position).length();
						if d > 1. && d < 50. && segment.flags.contains(segment::TORSO) {
							let r = segment.mesh.shape.radius();
							agent.state.absorb(self.dt * r * r / d * d);
						}
						agent.state.consume(self.dt * segment.state.get_charge());
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
			source: Position::zero(),
		}
	}
}

impl AlifeSystem {
	pub fn source(&mut self, pos: Position) {
		self.source = pos;
	}
}
