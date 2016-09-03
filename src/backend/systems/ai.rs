use super::*;
use std::collections::HashMap;
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

type IdPositionMap = HashMap<obj::Id, Position>;

pub struct AiSystem {
	remote: Box<[Position]>,
	targets: IdPositionMap,
}

impl Updateable for AiSystem {}

impl System for AiSystem {
	fn from_world(&mut self, world: &world::World) {
		self.remote = world.emitters().iter().map(|e| e.transform().position).collect::<Vec<_>>().into_boxed_slice();
		self.targets = world.agents(agent::AgentType::Resource)
			.iter()
			.filter(|&(_, ref v)| v.state.is_active())
			.map(|(_, v)| (v.id(), v.transform().position))
			.collect::<HashMap<_, _>>();
	}

	fn to_world(&self, world: &mut world::World) {
		Self::update_minions(&self.targets,
		                     &mut world.agents_mut(agent::AgentType::Minion));
	}
}

impl Default for AiSystem {
	fn default() -> Self {
		AiSystem {
			remote: Box::new([]),
			targets: HashMap::new(),
		}
	}
}

impl AiSystem {
	fn update_minions(targets: &IdPositionMap, minions: &mut agent::AgentMap) {
		for (_, agent) in minions.iter_mut() {
			let brain = agent.brain();
			let head = agent.first_segment(segment::SENSOR);
			if let Some(sensor) = head {
				let p0 = sensor.transform.position;
				let radar_range = sensor.mesh.shape.radius() * 10.;
				let current_target = agent.state.target().clone();
				let current_target_position = agent.state.target_position().clone();

				let new_target: Option<(obj::Id, Position)> = match current_target {
					None => {
						targets.iter()
							.find(|&(_, &p)| (p - p0).length() < radar_range)
							.map(|(&id, &position)| (id, position))
					}
					Some(id) => targets.get(&id).map(|&position| (id, position)),
				};

				match new_target {
					None => agent.state.retarget(None, current_target_position),
					Some((id, position)) => agent.state.retarget(Some(id), position),
				};

				let target_position = agent.state.target_position().clone();

				let segments = &mut agent.segments_mut();
				let t = target_position - sensor.transform.position;
				let d = t.length();
				for segment in segments.iter_mut() {
					if segment.flags.intersects(segment::ACTUATOR) {
						let power = segment.state.get_charge() * segment.mesh.shape.radius().powi(2);
						let f: Position = Matrix2::from_angle(rad(segment.transform.angle)) * Position::unit_y();
						let proj = t.dot(f);
						let intent = if let Some(refs) = segment.state.last_touched {
							match refs.id().type_of() {
								agent::AgentType::Resource => Intent::Idle,
								_ => Intent::RunAway(f.normalize_to(power * brain.timidity)),
							}
						} else if segment.flags.contains(segment::RUDDER) && proj > 0. && d > brain.focus &&
						                d < brain.caution {
							Intent::Move(f.normalize_to(power * brain.hunger))
						} else if segment.flags.contains(segment::THRUSTER) && proj > 0. && d > brain.curiosity {
							Intent::Move(f.normalize_to(power * brain.haste))
						} else if segment.flags.contains(segment::BRAKE) && (proj < 0. || d < brain.fear) {
							Intent::Move(f.normalize_to(-power * brain.prudence))
						} else {
							Intent::Idle
						};
						match intent {
							Intent::Idle => segment.state.set_target_charge(brain.rest),
							Intent::Move(_) => segment.state.set_target_charge(brain.thrust),
							Intent::RunAway(_) => segment.state.set_charge(brain.thrust),
						}
						segment.state.intent = intent;
					}
				}
			}
		}
	}
}
