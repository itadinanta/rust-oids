pub mod physics;
pub mod animation;
pub mod ai;
pub mod alife;
pub mod game;
pub mod particle;

pub use self::physics::PhysicsSystem;
pub use self::animation::AnimationSystem;
pub use self::game::GameSystem;
pub use self::ai::AiSystem;
pub use self::alife::AlifeSystem;
pub use self::particle::ParticleSystem;

use backend::world;
use backend::messagebus::{PubSub, Outbox};

use core::clock::Seconds;

pub trait System {
	fn attach(&mut self, _: &mut PubSub) {}
	fn init(&mut self, _: &world::World) {}
	fn register(&mut self, _: &world::agent::Agent) {}
	fn unregister(&mut self, _: &world::agent::Agent) {}
	fn import(&mut self, _: &world::World) {}
	fn update(&mut self, _world_state: &world::AgentState, _dt: Seconds) {}
	fn export(&self, _: &mut world::World, _: &Outbox) {}

	fn step(&mut self, world: &world::World, dt: Seconds) {
		self.import(world);
		self.update(world, dt)
	}

	fn apply(&self, world: &mut world::World, outbox: &Outbox) {
		self.export(world, outbox)
	}
}
