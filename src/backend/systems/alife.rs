use super::*;
use std::collections::HashMap;
use core::geometry;
use backend::obj;
use backend::obj::Transformable;
use backend::obj::Identified;
use backend::world;
use backend::world::gen;
use backend::world::agent;
use backend::world::segment;
use backend::world::WorldState;

type EatenMap = HashMap<obj::Id, agent::State>;

pub struct AlifeSystem {
	dt: f32,
	source: Box<[world::Emitter]>,
	eaten: EatenMap,
}

impl Updateable for AlifeSystem {
	fn update(&mut self, _: &WorldState, dt: f32) {
		self.dt = dt;
	}
}

impl System for AlifeSystem {
	fn from_world(&mut self, world: &world::World) {
		self.source = world.emitters().to_vec().into_boxed_slice();
		self.eaten = Self::find_eaten_resources(&world.agents(agent::AgentType::Minion),
		                                        &world.agents(agent::AgentType::Resource));
	}

	fn to_world(&self, world: &mut world::World) {
		Self::update_resources(self.dt,
		                       &mut world.agents_mut(agent::AgentType::Resource),
		                       &self.eaten);
		let (spores, corpses) = Self::update_minions(self.dt,
		                                             &world.extent.clone(),
		                                             &mut world.agents_mut(agent::AgentType::Minion),
		                                             &self.eaten);
		let hatch = Self::update_spores(self.dt, &mut world.agents_mut(agent::AgentType::Spore));

		for &(ref transform, ref dna) in spores.into_iter() {
			world.new_spore(*transform, dna);
		}
		for &(ref transform, ref dna) in hatch.into_iter() {
			world.hatch_spore(*transform, dna);
		}
		for &(ref transform, ref dna) in corpses.into_iter() {
			world.decay_to_resource(*transform, dna);
		}
	}
}

impl Default for AlifeSystem {
	fn default() -> Self {
		AlifeSystem {
			dt: 1. / 60.,
			source: Box::new([]),
			eaten: EatenMap::new(),
		}
	}
}

impl AlifeSystem {
	fn find_eaten_resources(minions: &agent::AgentMap, resources: &agent::AgentMap) -> EatenMap {
		let mut eaten = HashMap::new();
		for (_, agent) in minions.iter() {
			if agent.state.is_active() {
				for segment in agent.segments.iter() {
					if segment.flags.contains(segment::MOUTH) {
						if let Some(key) = segment.state.last_touched {
							if let Some(&agent::Agent { ref state, .. }) = resources.get(&key.id()) {
								eaten.insert(key.id(), (*state).clone());
							}
						}
					}
				}
			}
		}
		eaten
	}

	fn update_minions(dt: f32,
	                  extent: &geometry::Rect,
	                  minions: &mut agent::AgentMap,
	                  eaten: &EatenMap)
	                  -> (Box<[(geometry::Transform, gen::Dna)]>, Box<[(geometry::Transform, gen::Dna)]>) {
		let mut spawns = Vec::new();
		let mut corpses = Vec::new();
		for (key, agent) in minions.iter_mut() {
			if agent.state.is_active() {
				if agent.state.lifespan().is_expired() && agent.state.consume(50.) {
					spawns.push((agent.last_segment().transform(), agent.dna().clone()));
					agent.state.renew();
				}

				for segment in agent.segments.iter_mut() {
					let p = segment.transform().position;
					if p.x < extent.min.x || p.x > extent.max.x || p.y < extent.min.y || p.y > extent.max.y {
						agent.state.die();
					}
					if segment.flags.contains(segment::MOUTH) {
						if let Some(id) = segment.state.last_touched {
							if let Some(state) = eaten.get(&id.id()) {
								agent.state.absorb(5. * state.power());
								println!("Agent {} state is {:?}", key, agent.state);
							}
						}
					}
					agent.state.consume(dt * segment.state.get_charge() * segment.mesh.shape.radius());
					segment.state.update(dt);
				}

				if agent.state.power() < 1. {
					for segment in agent.segments.iter().filter(|s| s.flags.contains(segment::MIDDLE)) {
						corpses.push((segment.transform, agent.dna().clone()));
					}
					agent.state.die();
				}
			}
		}
		(spawns.into_boxed_slice(), corpses.into_boxed_slice())
	}

	fn update_resources(dt: f32, resources: &mut agent::AgentMap, eaten: &EatenMap) {
		for (_, agent) in resources.iter_mut() {
			if eaten.get(&agent.id()).is_some() {
				agent.state.die();
			} else if agent.state.lifespan().is_expired() {
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
}
