pub mod physics;
pub mod animation;
pub mod ai;
pub mod alife;
pub mod game;
pub mod audio;

pub use self::physics::PhysicsSystem;
pub use self::animation::AnimationSystem;
pub use self::game::GameSystem;
pub use self::ai::AiSystem;
pub use self::alife::AlifeSystem;
pub use self::audio::AudioSystem;

use backend::world;

pub trait Updateable {
	fn update(&mut self, _world_state: &world::WorldState, _dt: f32) {}
}

pub trait System: Updateable {
	fn init(&mut self, _: &world::World) {}
	fn register(&mut self, _: &world::agent::Agent) {}
	fn unregister(&mut self, _: &world::agent::Agent) {}
	fn from_world(&mut self, _: &world::World) {}
	fn to_world(&self, _: &mut world::World) {}
	fn update_world(&mut self, world: &mut world::World, dt: f32) {
		self.from_world(world);
		self.update(world, dt);
		self.to_world(world);
	}
}
