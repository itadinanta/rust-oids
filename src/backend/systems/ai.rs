use super::*;
use backend::world;
use backend::world::WorldState;
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
		let target = self.remote;
		for (_, agent) in world.minions.agents_mut() {
			let segments = &mut agent.segments_mut();
			let order = segments.len() as f32;
			let d0 = 2. * order;
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
						let intent = if segment.flags.contains(world::RUDDER) && proj > 0. && d > d0 * 1.2 &&
						                d < d0 * 2.0 {
							Some(f.normalize_to(power * 4. * order))
						} else if segment.flags.contains(world::THRUSTER) && proj > 0. && d > d0 * 1.1 {
							Some(f.normalize_to(power * 2. * order))
						} else if segment.flags.contains(world::BRAKE) && (proj < 0. || d < d0 * 0.9) {
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
