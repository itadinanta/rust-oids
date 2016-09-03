use super::*;
use backend::obj;
use backend::obj::Identified;
use backend::obj::Transformable;
use backend::world;
use backend::world::agent;
use backend::world::agent::TypedAgent;
use backend::world::segment;
use backend::world::segment::Intent;
use cgmath::*;
use core::geometry::Position;

pub struct AiSystem {
	remote: Position,
}

impl Updateable for AiSystem {}

impl System for AiSystem {
	fn from_world(&mut self, world: &world::World) {
		let emitters = world.emitters();
		if !emitters.is_empty() {
			self.remote = emitters[0].transform().position;
		}
	}

	fn to_world(&self, world: &mut world::World) {
		let eaten = Self::update_minions(&self.remote,
		                                 &mut world.agents_mut(agent::AgentType::Minion));
		Self::update_resources(&eaten, &mut world.agents_mut(agent::AgentType::Resource))
	}
}

impl Default for AiSystem {
	fn default() -> Self {
		AiSystem { remote: Position::zero() }
	}
}

impl AiSystem {
	fn update_minions(target: &Position, minions: &mut agent::AgentMap) -> Box<[obj::Id]> {
		let mut eaten = Vec::new();
		for (_, agent) in minions.iter_mut() {
			let brain = agent.brain();
			let mut absorb: f32 = 0.;
			{
				let segments = &mut agent.segments_mut();
				if let Some(sensor) = segments.iter()
					.find(|segment| segment.flags.contains(segment::SENSOR))
					.map(|sensor| sensor.clone()) {
					let t = target - sensor.transform.position;
					let d = t.length();
					for segment in segments.iter_mut() {
						if segment.flags.intersects(segment::ACTUATOR) {
							let power = segment.state.get_charge() * segment.mesh.shape.radius().powi(2);
							let f: Position = Matrix2::from_angle(rad(segment.transform.angle)) * Position::unit_y();
							let proj = t.dot(f);
							let intent = if let Some(refs) = segment.state.collision_detected {
								match refs.id().type_of() {
									agent::AgentType::Resource => {
										if segment.flags.contains(segment::MOUTH) {
											Intent::Eat(refs.id())
										} else {
											Intent::RunAway(f.normalize_to(power * brain.timidity))
										}
									}
									_ => Intent::RunAway(f.normalize_to(power * brain.timidity)),
								}
							} else if segment.flags.contains(segment::RUDDER) && proj > 0. &&
							                d > brain.focus && d < brain.caution {
								Intent::Move(f.normalize_to(power * brain.hunger))
							} else if segment.flags.contains(segment::THRUSTER) && proj > 0. &&
							                d > brain.curiosity {
								Intent::Move(f.normalize_to(power * brain.haste))
							} else if segment.flags.contains(segment::BRAKE) && (proj < 0. || d < brain.fear) {
								Intent::Move(f.normalize_to(-power * brain.prudence))
							} else {
								Intent::Idle
							};
							match intent {
								Intent::Idle => segment.state.set_target_charge(brain.rest),
								Intent::Eat(id) => {
									eaten.push(id);
									absorb += 10.0;
									segment.state.set_target_charge(brain.thrust);
								}
								Intent::Move(_) => segment.state.set_target_charge(brain.thrust),
								Intent::RunAway(_) => segment.state.set_charge(brain.thrust),
							}
							segment.state.intent = intent;
						}
					}
				}
			}
			if absorb > 0. {
				agent.state.absorb(absorb);
			}
		}
		eaten.into_boxed_slice()
	}

	fn update_resources(ids: &[obj::Id], resources: &mut agent::AgentMap) {
		// TODO: ai shouldn't change state here, this should be done in alife
		for id in ids {
			if let Some(resource) = resources.get_mut(id) {
				resource.state.die();
			}
		}
	}
}
