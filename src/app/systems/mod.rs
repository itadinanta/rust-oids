pub mod physics;

pub use self::physics::PhysicsSystem;

use app::world;

pub trait System {
	fn register(&mut self, creature: &world::Creature);
	fn update(&mut self, dt: f32);
	fn to_world(&self, world: &mut world::World);
}
