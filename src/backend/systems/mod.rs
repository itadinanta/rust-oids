pub mod physics;
pub mod animation;
pub mod ai;
pub mod game;

pub use self::physics::PhysicsSystem;
pub use self::animation::AnimationSystem;
pub use self::game::GameSystem;
pub use self::ai::AiSystem;

use backend::world;

pub trait Updateable {
	fn update(&mut self, state: &world::WorldState, dt: f32);
}

pub trait System: Updateable {
	fn init(&mut self, world: &world::World);
	fn register(&mut self, creature: &world::Agent);
	fn from_world(&self, world: &world::World);
	fn to_world(&self, world: &mut world::World);
	fn update_world(&mut self, dt: f32, world: &mut world::World) {
		self.from_world(world);
		self.update(world, dt);
		self.to_world(world);
	}
}
