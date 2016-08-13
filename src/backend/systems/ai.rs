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
			let segments = agent.segments_mut();
			// let torso = segments[0].unwrap();
			for segment in segments {
				if segment.flags.intersects(world::RUDDER | world::THRUSTER) {
					let center: Position = segment.transform.position;
					let power = segment.state.charge * segment.mesh.shape.radius().powi(2);
					let facing: Position = Matrix2::from_angle(rad(segment.transform.angle)) * Position::unit_y();
					let intent = if segment.flags.contains(world::RUDDER) {
						let t = self.remote - center;
						let f = -facing;

						if f.dot(facing) > 0. {
							Some(facing.normalize_to(-power * 10.))
						} else {
							None
						}
					} else if segment.flags.contains(world::THRUSTER) {
						Some(facing.normalize_to(power * 50.))
					} else {
						None
					};
					segment.state.intent = intent;
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
