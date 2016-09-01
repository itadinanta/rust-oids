use super::*;
use backend::obj::Transformable;
use backend::world;
use backend::world::gen;
use backend::world::agent;
use backend::world::segment;
use backend::world::WorldState;
use cgmath::*;
use core::geometry;

pub struct AlifeSystem {
	dt: f32,
	source: geometry::Position,
}

impl Updateable for AlifeSystem {
	fn update(&mut self, _: &WorldState, dt: f32) {
		self.dt = dt;
	}
}

impl System for AlifeSystem {
	fn to_world(&self, world: &mut world::World) {
		Self::update_resources(self.dt, &mut world.agents_mut(agent::AgentType::Resource));
		let spores = Self::update_minions(self.dt,
		                                  &self.source,
		                                  &mut world.agents_mut(agent::AgentType::Minion));
		let hatch = Self::update_spores(self.dt, &mut world.agents_mut(agent::AgentType::Spore));

		for &(ref transform, ref dna) in spores.into_iter() {
			world.new_spore(*transform, dna);
		}
		for &(ref transform, ref dna) in hatch.into_iter() {
			world.hatch_spore(*transform, dna);
		}
	}
}

impl Default for AlifeSystem {
	fn default() -> Self {
		AlifeSystem {
			dt: 1. / 60.,
			source: geometry::Position::zero(),
		}
	}
}

impl AlifeSystem {
	fn update_minions(dt: f32,
	                  source: &geometry::Position,
	                  minions: &mut agent::AgentMap)
	                  -> Box<[(geometry::Transform, gen::Dna)]> {
		let mut spawns = Vec::new();
		for (_, agent) in minions.iter_mut() {
			if agent.state.is_active() {
				if agent.state.lifespan().is_expired() || agent.state.power() <= 0. {
					agent.state.die();
				} else if agent.state.lifespan().left() < 5. && agent.state.consume(10.) {
					spawns.push((agent.last_segment().transform(), agent.dna().clone()));
				} else {
					for segment in agent.segments.iter_mut() {
						// some source of food, let's use the light for now
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
		spawns.into_boxed_slice()
	}

	fn update_resources(dt: f32, resources: &mut agent::AgentMap) {
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

	fn update_spores(dt: f32, resources: &mut agent::AgentMap) -> Box<[(geometry::Transform, gen::Dna)]> {
		let mut spawns = Vec::new();
		for (_, agent) in resources.iter_mut() {
			if agent.state.lifespan().is_expired() {
				agent.state.die();
				spawns.push((agent.transform(), agent.dna().clone()))
			} else if agent.state.is_active() {
				for segment in agent.segments.iter_mut() {
					segment.state.update(dt)
				}
			}
		}
		spawns.into_boxed_slice()
	}


	pub fn source(&mut self, pos: geometry::Position) {
		self.source = pos;
	}
}
