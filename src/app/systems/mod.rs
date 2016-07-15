pub mod physics;
pub mod animation;

pub use self::physics::PhysicsSystem;
pub use self::animation::AnimationSystem;

use app::world;
use app::obj;

pub trait System: obj::Updateable {
	fn register(&mut self, creature: &world::Creature);
	fn to_world(&self, world: &mut world::World);
	fn update_world(&mut self, dt: f32, world: &mut world::World) {
		self.update(dt);
		self.to_world(world);
	}
}
