use super::*;
use backend::obj;
use core::geometry::Transform;
use core::geometry::Position;
use core::math::*;
use core::clock::*;
use backend::world;

type Phase = f32;

struct Particle {
	transform: Transform,
	prev_transform: Transform,
	velocity: Transform,
	acceleration: Transform,
	ttl: Seconds,
}

impl Default for Particle {
	fn default() -> Particle {
		Particle {
			transform: Transform::default(),
			prev_transform: Transform::default(),
			velocity: Transform::from_position(-Position::unit_x()),
			acceleration: Transform::default(),
			ttl: Seconds::new(1.0),
		}
	}
}

trait Emitter {
	fn emit<V>(&mut self, destination: &mut V) where V: Extend<Particle>;
}

struct SimpleEmitter {
	transform: Transform,
	attached_to: Option<obj::Id>,
	phase: Phase,
	rate: f32,
	cluster_size: usize,
}

impl Emitter for SimpleEmitter {
	fn emit<V>(&mut self, destination: &mut V) where V: Extend<Particle> {
		let fill = vec![Particle::default()];
		destination.extend(fill);
	}
}

use super::*;
use std::rc::Rc;
use std::cell::RefCell;
use backend::world::WorldState;
use backend::world::agent;
use core::clock::*;
use num_traits::clamp;

#[allow(unused)]
pub struct ParticleSystem {
	particles: Vec<Particle>,
	emitters: Vec<SimpleEmitter>,
	dt: Seconds,
}

impl Updateable for ParticleSystem {
	fn update(&mut self, _: &WorldState, dt: Seconds) {
		self.dt = dt;
	}
}

impl System for ParticleSystem {
	fn put_to_world(&self, world: &mut world::World) {
		world.clear_particles();
		for particle in &self.particles {
			world.add_particle(world::particle::Particle::new(
				particle.transform.clone(),
				particle.prev_transform.clone(),
			));
		}
	}
}

impl Default for ParticleSystem {
	fn default() -> Self {
		ParticleSystem {
			particles: Vec::new(),
			emitters: Vec::new(),
			dt: Seconds::new(0.),
		}
	}
}

impl ParticleSystem {}
