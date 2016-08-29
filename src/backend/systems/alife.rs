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
		Self::update_resources(self.dt, &mut world.agents_mut(world::AgentType::Resource));
		Self::update_minions(self.dt,
		                     &self.source,
		                     &mut world.agents_mut(world::AgentType::Minion));
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
	fn update_minions(dt: f32, source: &Position, minions: &mut world::AgentMap) {
		for (_, agent) in minions.iter_mut() {
			if agent.state.is_active() {
				if agent.state.power() > 0. {
					agent.state.renew();
				}
				if agent.state.lifespan().is_expired() {
					agent.state.die();
				} else {
					for segment in agent.segments.iter_mut() {
						// some source of food, let's use the light
						let d = (source - segment.transform.position).length();
						if d > 1. && d < 50. && segment.flags.contains(segment::TORSO) {
							let r = segment.mesh.shape.radius();
							agent.state.absorb(dt * r * r / d * d);
						}
						agent.state.consume(dt * segment.state.get_charge());
						segment.state.update(dt);
					}
				}
			}
		}
	}

	fn update_resources(dt: f32, resources: &mut world::AgentMap) {
		for (_, agent) in resources.iter_mut() {
			if agent.state.lifespan().is_expired() {
				agent.state.die();
			} else if agent.state.is_active() {
				for segment in agent.segments.iter_mut() {
					segment.state.update(dt)
				}
			}
		}
	}

	pub fn source(&mut self, pos: Position) {
		self.source = pos;
	}
}
