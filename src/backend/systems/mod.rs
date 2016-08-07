pub mod physics;
pub mod animation;

pub use self::physics::PhysicsSystem;
pub use self::animation::AnimationSystem;

use backend::world;

pub trait Updateable {
	fn update(&mut self, state: &world::WorldState, dt: f32);
}

pub trait System: Updateable {
	fn register(&mut self, creature: &world::Creature);
	fn from_world(&self, world: &world::World);
	fn to_world(&self, world: &mut world::World);
	fn update_world(&mut self, dt: f32, world: &mut world::World) {
		self.from_world(world);
		self.update(world, dt);
		self.to_world(world);
	}
}
