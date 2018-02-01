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
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use backend::world::WorldState;
use backend::world::agent;
use std::iter::Iterator;
use core::clock::*;
use num_traits::clamp;
use cgmath::InnerSpace;

type Phase = f32;

const TRAIL_LENGTH: usize = 6;

struct Particle {
	id: obj::Id,
	transform: Transform,
	motion: Motion,
	acceleration: Velocity,
	trail: VecDeque<Position>,
	dampening: f32,
	friction: f32,
	ttl: Seconds,
}

impl Particle {
	fn new(id: obj::Id) -> Particle {
		Particle {
			id: 0,
			transform: Transform::default(),
			trail: VecDeque::with_capacity(TRAIL_LENGTH),
			motion: Motion::new(Velocity::unit_x(), 0.),
			dampening: 0.,
			friction: 0.,
			acceleration: Velocity::zero(), // in the frame reference of velocity
			ttl: Seconds::new(1.0),
		}
	}
}

trait Emitter {
	fn emit(&mut self, id_counter: &mut usize, destination: &mut HashMap<obj::Id, Particle>);
}

struct SimpleEmitter {
	transform: Transform,
	motion: Motion,
	attached_to: Option<obj::Id>,
	phase: Phase,
	rate: f32,
	cluster_size: usize,
	active: bool,
}

impl SimpleEmitter {
	fn new() -> SimpleEmitter {
		SimpleEmitter {
			transform: Transform::new(Position::zero(), 0.),
			motion: Motion::new(10. * Velocity::unit_x(), 0.),
			attached_to: None,
			phase: 0.,
			rate: 5.,
			cluster_size: 1,
			active: true,
		}
	}
}

impl Emitter for SimpleEmitter {
	fn emit(&mut self, id_counter: &mut usize, destination: &mut HashMap<obj::Id, Particle>) {
		for i in 0..self.cluster_size {
			let id = *id_counter;
			destination.insert(id, Particle {
				id,
				transform: self.transform.clone(),
				trail: VecDeque::new(),
				motion: self.motion.clone(),
				dampening: 0.,
				friction: 0.,
				acceleration: -Velocity::unit_y(),
				ttl: seconds(60.),
			});
			*id_counter = id + 1;
		}
	}
}

#[allow(unused)]
pub struct ParticleSystem {
	id_counter: usize,
	trail_length: usize,
	particles: HashMap<usize, Particle>,
	emitters: Vec<SimpleEmitter>,
	dt: Seconds,
	simulation_timer: SharedTimer<SimulationTimer>,
	simulation_clock: TimerStopwatch<SimulationTimer>,
}

impl Updateable for ParticleSystem {
	fn update(&mut self, _: &WorldState, dt: Seconds) {
		self.dt = dt;
		self.simulation_timer.borrow_mut().tick(dt);

		for emitter in &mut self.emitters {
			if emitter.active {
				let dt = dt.get() as f32;
				let phase = emitter.phase;
				emitter.phase = (emitter.phase + dt * emitter.rate) % 1.;
				if emitter.phase < phase {
					emitter.emit(&mut self.id_counter, &mut self.particles);
				}
			}
		}

		let mut expired: Vec<usize> = Vec::new();

		for (id, particle) in self.particles.iter_mut() {
			particle.ttl -= dt;
			let dt = dt.get() as f32;
			if particle.ttl.get() <= 0. {
				expired.push(*id);
			} else {
				let axis_x = particle.motion.velocity.normalize();
				let axis_y = Velocity::new(-axis_x.y, axis_x.x);
				let world_acceleration = particle.acceleration.x * axis_x
					+ particle.acceleration.y * axis_y;
				if particle.trail.len() > TRAIL_LENGTH {
					particle.trail.pop_front();
				}
				particle.trail.push_back(particle.transform.position);
				particle.motion.velocity += dt * world_acceleration;
				particle.transform.position += dt * particle.motion.velocity;
				particle.transform.angle += dt * particle.motion.spin;
				particle.motion.velocity *= 1. - (dt * particle.friction);
				particle.acceleration *= 1. - (dt * particle.dampening);
			}
		}

		for id in expired {
			self.particles.remove(&id);
		}
	}
}

impl System for ParticleSystem {
	fn put_to_world(&self, world: &mut world::World) {
		world.clear_particles();
		for (_, particle) in &self.particles {
			world.add_particle(world::particle::Particle::new(
				particle.transform.clone(),
				particle.motion.velocity.normalize(),
				particle.trail
					.iter()
					.map(|t| *t)
					.collect::<Vec<_>>().into_boxed_slice(),
			));
		}
	}

	fn get_from_world(&mut self, world: &world::World) {
		if self.emitters.is_empty() {
			self.emitters.push(SimpleEmitter::new());
		}
		for emitter in &mut self.emitters {
			if let Some(attached_to) = emitter.attached_to {
				if let Some(ref agent) = world.agent(attached_to) {
					if let Some(segment) = agent.segment(0) {
						emitter.transform = segment.transform.clone();
						if let Some(ref motion) = segment.motion {
							emitter.motion = motion.clone();
						}
					}
					//emitter.motion = agent.state.motion;
				}
			}
		}
	}
}

impl Default for ParticleSystem {
	fn default() -> Self {
		let simulation_timer = Rc::new(RefCell::new(SimulationTimer::new()));
		ParticleSystem {
			id_counter: 0,
			trail_length: TRAIL_LENGTH,
			particles: HashMap::new(),
			emitters: vec![SimpleEmitter::new()],
			dt: seconds(0.),
			simulation_clock: TimerStopwatch::new(simulation_timer.clone()),
			simulation_timer,
		}
	}
}

impl ParticleSystem {}
