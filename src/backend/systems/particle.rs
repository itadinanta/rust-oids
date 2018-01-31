use super::*;
use backend::obj;
use core::geometry::Transform;
use core::geometry::Motion;
use core::geometry::Position;
use core::geometry::Velocity;
use num::Zero;
use core::math::*;
use core::clock::*;
use backend::world;
use std::collections::VecDeque;
use std::rc::Rc;
use std::cell::RefCell;
use backend::world::WorldState;
use backend::world::agent;
use core::clock::*;
use num_traits::clamp;
use cgmath::InnerSpace;

type Phase = f32;

const TRAIL_LENGTH: usize = 6;

struct Particle {
	position: Transform,
	velocity: Motion,
	acceleration: Velocity,
	trail: VecDeque<Position>,
	dampening: f32,
	friction: f32,
	ttl: Seconds,
	active: bool,
}

impl Default for Particle {
	fn default() -> Particle {
		Particle {
			position: Transform::default(),
			trail: VecDeque::with_capacity(TRAIL_LENGTH),
			velocity: Motion::new(Velocity::unit_x(), 0.),
			dampening: 0.,
			friction: 0.,
			acceleration: Velocity::zero(), // in the frame reference of velocity
			ttl: Seconds::new(1.0),
			active: true,
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

#[allow(unused)]
pub struct ParticleSystem {
	trail_length: usize,
	particles: Vec<Particle>,
	emitters: Vec<SimpleEmitter>,
	dt: Seconds,
	simulation_timer: SharedTimer<SimulationTimer>,
	simulation_clock: TimerStopwatch<SimulationTimer>,
}

impl Updateable for ParticleSystem {
	fn update(&mut self, _: &WorldState, dt: Seconds) {
		self.dt = dt;
		self.simulation_timer.borrow_mut().tick(dt);

		for particle in &mut self.particles {
			particle.ttl -= dt;
			if particle.ttl.get() <= 0. {
				particle.active = false;
				let axis_x = particle.velocity.velocity.normalize();
				let axis_y = Velocity::new(-axis_x.y, axis_x.x);
				let world_acceleration = particle.acceleration.x * axis_x
					+ particle.acceleration.y * axis_y;
				let dt = dt.get() as f32;
				particle.velocity.velocity += dt * world_acceleration;
				particle.position.position += dt * particle.velocity.velocity;
				particle.position.angle += dt * particle.velocity.spin;
				particle.velocity.velocity *= 1. - (dt * particle.friction);
				particle.acceleration *= 1. - (dt * particle.dampening);
			}
		}
	}
}

impl System for ParticleSystem {
	fn put_to_world(&self, world: &mut world::World) {
		world.clear_particles();
		for particle in &self.particles {
			world.add_particle(world::particle::Particle::new(
				particle.position.clone(),
				particle.velocity.velocity.normalize(),
				particle.trail
					.iter()
					.map(|t| *t)
					.collect::<Vec<_>>().into_boxed_slice(),
			));
		}
	}
}

impl Default for ParticleSystem {
	fn default() -> Self {
		let simulation_timer = Rc::new(RefCell::new(SimulationTimer::new()));
		ParticleSystem {
			trail_length: TRAIL_LENGTH,
			particles: Vec::new(),
			emitters: Vec::new(),
			dt: Seconds::new(0.),
			simulation_clock: TimerStopwatch::new(simulation_timer.clone()),
			simulation_timer,
		}
	}
}

impl ParticleSystem {}
