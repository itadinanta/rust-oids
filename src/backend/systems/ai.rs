use super::*;
use backend::world;
use backend::world::WorldState;
use std::time::*;
use backend::obj;
use cgmath::*;
use core::geometry::Position;

pub struct AiSystem {
	remote: Position,
}

impl Updateable for AiSystem {
	fn update(&mut self, state: &WorldState, dt: f32) {}
}

impl System for AiSystem {
	fn init(&mut self, world: &world::World) {}

	fn register(&mut self, _: &world::Agent) {}

	fn from_world(&self, world: &world::World) {}

	fn to_world(&self, world: &mut world::World) {
		for (_, agent) in world.minions.agents_mut() {
			// let torso = segments[0].unwrap();
			// 			let sensor = {
			// 				agent.segments_mut().find(|segment| segment.flags.contains(world::SENSOR))
			// 			};
			let segments = &mut agent.segments_mut();
			let order = segments.len() as f32;
			if let Some(sensor) = segments.iter()
				.find(|segment| segment.flags.contains(world::HEAD))
				.map(|sensor| sensor.clone()) {
				for segment in segments.iter_mut() {
					if segment.flags.intersects(world::ACTUATOR) {
						let power = segment.state.charge * segment.mesh.shape.radius().powi(2);
						let f: Position = Matrix2::from_angle(rad(segment.transform.angle)) * Position::unit_y();
						let t = self.remote - sensor.transform.position;
						let d = t.length();
						let intent = if segment.flags.contains(world::RUDDER) && t.dot(f) > 0. && t.length() > 10 {
							Some(f.normalize_to(power * 4. * order))
						} else if segment.flags.contains(world::THRUSTER) && t.dot(f) > 0. && d > 10.5 {
							Some(f.normalize_to(power * 2. * order))
						} else if segment.flags.contains(world::BRAKE) && (t.dot(f) < 0. || d < 9.5) {
							Some(f.normalize_to(-power * 3. * order))
						} else {
							None
						};
						match intent {
							None => segment.state.target_charge = 0.1,
							Some(_) => segment.state.target_charge = 0.5,
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
