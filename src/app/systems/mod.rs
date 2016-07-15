pub mod physics;

pub use self::physics::PhysicsSystem;

use app::world;

pub trait System {
	fn update(&mut self, dt: f32);
	fn register(&mut self, creature: &world::Creature);
	fn to_world(&self, world: &mut world::World);
}
