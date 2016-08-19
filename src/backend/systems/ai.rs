use super::*;
use backend::world;
use backend::world::Intent;
use backend::world::WorldState;
use cgmath::*;
use core::geometry::Position;

pub struct AiSystem {
	remote: Position,
}

impl Updateable for AiSystem {
	fn update(&mut self, _: &WorldState, dt: f32) {}
}

impl System for AiSystem {
	fn to_world(&self, world: &mut world::World) {
		let target = self.remote;
		for (_, agent) in world.minions.agents_mut() {
			let brain = agent.brain();
			let segments = &mut agent.segments_mut();
			if let Some(sensor) = segments.iter()
				.find(|segment| segment.flags.contains(world::SENSOR))
				.map(|sensor| sensor.clone()) {
				let t = target - sensor.transform.position;
				let d = t.length();
				for segment in segments.iter_mut() {
					if segment.flags.intersects(world::ACTUATOR) {
						let power = segment.state.charge * segment.mesh.shape.radius().powi(2);
						let f: Position = Matrix2::from_angle(rad(segment.transform.angle)) * Position::unit_y();
						let proj = t.dot(f);
						let intent = if segment.state.collision_detected {
							Intent::RunAway(f.normalize_to(power * brain.timidity))
						} else if segment.flags.contains(world::RUDDER) && proj > 0. && d > brain.focus &&
						                d < brain.caution {
							Intent::Move(f.normalize_to(power * brain.hunger))
						} else if segment.flags.contains(world::THRUSTER) && proj > 0. && d > brain.curiosity {
							Intent::Move(f.normalize_to(power * brain.haste))
						} else if segment.flags.contains(world::BRAKE) && (proj < 0. || d < brain.fear) {
							Intent::Move(f.normalize_to(-power * brain.prudence))
						} else {
							Intent::Idle
						};
						match intent {
							Intent::Idle => segment.state.target_charge = brain.rest,
							Intent::Move(_) => segment.state.target_charge = brain.thrust,
							Intent::RunAway(_) => segment.state.charge = brain.thrust,
						}
						segment.state.intent = intent;
					}
				}
			}
		}
	}
}

impl AiSystem {
	pub fn new() -> Self {
		AiSystem { remote: Position::zero() }
	}

	pub fn follow_me(&mut self, pos: Position) {
		self.remote = pos;
	}
}
