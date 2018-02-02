use super::*;
use backend::obj;
use core::geometry::Transform;
use core::geometry::Motion;
use core::geometry::Position;
use core::geometry::Velocity;
use core::math;
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
use num;
use num::NumCast;
use cgmath::InnerSpace;
use rayon::prelude::*;

type Phase = f32;

const TRAIL_LENGTH: usize = 6;
const MAX_FADER: usize = 4;

struct Fader<S> {
	value: S,
	alpha: S,
	target: S,
}

impl<S> Fader<S> where S: num::Float {
	fn new(value: S, alpha: S, target: S) -> Fader<S> {
		Fader { value, alpha, target }
	}

	fn value(&self) -> S {
		self.value
	}

	fn with_target(&mut self, target: S) {
		self.target = target
	}

	fn update(&mut self, dt: Seconds) -> S {
		self.value = self.target + (self.value - self.target) * self.alpha * NumCast::from(dt.get()).unwrap();
		self.value
	}
}

type Tag = u64;

struct Particle {
	id: obj::Id,
	tag: Tag,
	transform: Transform,
	motion: Motion,
	acceleration: Velocity,
	trail: VecDeque<Position>,
	dampening: f32,
	friction: f32,
	faders: Vec<Fader<f32>>,
	ttl: Seconds,
}

trait Emitter {
	fn emit(&mut self, dt: Seconds, id_counter: &mut usize, destination: &mut HashMap<obj::Id, Particle>) -> bool;
	fn attached_to(&self) -> Option<obj::Id> { None }
	fn update_transform(&mut self, transform: Transform, motion: Option<Motion>) {}
}

struct SimpleEmitter {
	id: obj::Id,
	transform: Transform,
	motion: Motion,
	attached_to: Option<obj::Id>,
	phase: Phase,
	rate: f32,
	ttl: Option<Seconds>,
	cluster_size: usize,
	active: bool,
}

impl SimpleEmitter {
	fn new(id: obj::Id) -> SimpleEmitter {
		SimpleEmitter {
			id,
			transform: Transform::new(Position::zero(), 0.),
			motion: Motion::new(10. * Velocity::unit_x(), 0.),
			attached_to: None,
			phase: 0.,
			rate: 5.,
			ttl: None,
			cluster_size: 1,
			active: true,
		}
	}
}

impl Emitter for SimpleEmitter {
	fn emit(&mut self, dt: Seconds, id_counter: &mut usize, destination: &mut HashMap<obj::Id, Particle>) -> bool {
		if self.active {
			let phase = self.phase;
			self.phase = (self.phase + dt.get() as f32 * self.rate) % 1.;
			if self.phase < phase {
				for i in 0..self.cluster_size {
					let id = *id_counter;
					destination.insert(id, Particle {
						id,
						tag: 0,
						transform: self.transform.clone(),
						trail: VecDeque::new(),
						motion: self.motion.clone(),
						dampening: 0.,
						friction: 0.,
						faders: (0..MAX_FADER).map(|_| Fader::new(1.0, 0.5, 0.)).collect(),
						acceleration: -Velocity::unit_y(),
						ttl: seconds(60.),
					});
					*id_counter = id + 1;
				}
			}
		}
		if let Some(ttl) = self.ttl {
			let ttl = ttl - dt;
			self.ttl = Some(ttl);
			ttl.get() > 0.
		} else {
			true
		}
	}

	fn attached_to(&self) -> Option<obj::Id> {
		self.attached_to
	}

	fn update_transform(&mut self, transform: Transform, motion: Option<Motion>) {
		self.transform = transform.clone();
		if let Some(motion) = motion {
			self.motion = motion
		};
	}
}

#[allow(unused)]
pub struct ParticleSystem {
	id_counter: usize,
	trail_length: usize,
	particles: HashMap<obj::Id, Particle>,
	emitters: HashMap<obj::Id, Box<Emitter>>,
	dt: Seconds,
	simulation_timer: SharedTimer<SimulationTimer>,
	simulation_clock: TimerStopwatch<SimulationTimer>,
}

impl ParticleSystem {
	fn update_emitters(&mut self) {
		let dt = self.dt;
		let mut expired: Vec<usize> = Vec::new();

		for (id, emitter) in &mut self.emitters {
			let alive = emitter.emit(dt, &mut self.id_counter, &mut self.particles);
			if !alive {
				expired.push(*id);
			}
		}

		for id in expired {
			self.emitters.remove(&id);
		}
	}

	fn update_particles(&mut self) {
		let dt = self.dt;
		let expired: Vec<Option<obj::Id>> = self.particles
			.par_iter_mut().map(|(id, particle)| {
			particle.ttl -= dt;
			if particle.ttl.get() <= 0. {
				Some(*id)
			} else {
				{
					let dt = dt.get() as f32;
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
				for fader in particle.faders.iter_mut() { fader.update(dt); }
				None
			}
		}).collect();

		for id in expired.into_iter().filter(|expired| expired.is_some())
			.map(|expired| expired.unwrap()) {
			self.particles.remove(&id);
		}
	}
}

impl Updateable for ParticleSystem {
	fn update(&mut self, _: &WorldState, dt: Seconds) {
		self.dt = dt;
		self.simulation_timer.borrow_mut().tick(dt);

		self.update_emitters();
		self.update_particles();
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
			let emitter = SimpleEmitter::new(self.id_counter);
			self.emitters.insert(emitter.id, Box::new(emitter));
			self.id_counter += 1;
		}

		for (id, emitter) in self.emitters.iter_mut() {
			if let Some(attached_to) = emitter.attached_to() {
				if let Some(ref agent) = world.agent(attached_to) {
					if let Some(segment) = agent.segment(0) {
						emitter.update_transform(segment.transform.clone(), segment.motion.clone());
					}
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
			emitters: HashMap::new(),
			dt: seconds(0.),
			simulation_clock: TimerStopwatch::new(simulation_timer.clone()),
			simulation_timer,
		}
	}
}

