pub mod physics;
pub mod animation;
pub mod ai;
pub mod alife;
pub mod game;

pub use self::physics::PhysicsSystem;
pub use self::animation::AnimationSystem;
pub use self::game::GameSystem;
pub use self::ai::AiSystem;
pub use self::alife::AlifeSystem;
use backend::world;
use core::clock::Seconds;

pub trait Updateable {
	fn update(&mut self, _world_state: &world::WorldState, _dt: Seconds) {}
}

pub trait System: Updateable {
	fn init(&mut self, _: &world::World) {}
	fn register(&mut self, _: &world::agent::Agent) {}
	fn unregister(&mut self, _: &world::agent::Agent) {}
	fn get_from_world(&mut self, _: &world::World) {}
	fn put_to_world(&self, _: &mut world::World) {}
	fn update_world(&mut self, world: &mut world::World, dt: Seconds) {
		self.get_from_world(world);
		self.update(world, dt);
		self.put_to_world(world);
	}
}
