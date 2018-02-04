use super::*;
use backend::obj;
use core::geometry::Transform;
use core::geometry::Motion;
use core::geometry::Position;
use core::geometry::Velocity;
use num::Zero;
use core::clock::{seconds, Seconds, SimulationTimer, TimerStopwatch};
use backend::world;
use std::collections::VecDeque;
use std::collections::HashMap;
use backend::world::AgentState;
use std::iter::Iterator;
use std::f32::consts;
use num;
use num::NumCast;
use cgmath::InnerSpace;
use rayon::prelude::*;

type Phase = f32;

const TRAIL_LENGTH: usize = 6;
const MAX_FADER: usize = 4;

struct Fader<S> {
	value: S,
	fade_rate: S,
	target: S,
}

impl<S> Fader<S> where S: num::Float {
	fn new(value: S, fade_rate: S, target: S) -> Fader<S> {
		Fader { value, fade_rate, target }
	}

	fn value(&self) -> S {
		self.value
	}

	fn with_target(&mut self, target: S) {
		self.target = target
	}

	fn update(&mut self, dt: Seconds) -> S {
		self.value = self.value + (self.target - self.value) * self.fade_rate * NumCast::from(dt.get()).unwrap();
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
	trail_length: usize,
	trail: VecDeque<Position>,
	dampening: f32,
	friction: f32,
	faders: Vec<Fader<f32>>,
	ttl: Seconds,
}

trait Emitter {
	fn emit(&mut self, dt: Seconds, id_counter: &mut usize, destination: &mut HashMap<obj::Id, Particle>) -> bool;
	fn attached_to(&self) -> Option<obj::Id> { None }
	fn update_transform(&mut self, _transform: Transform, _motion: Option<Motion>) {}
}

struct SimpleEmitter {
	id: obj::Id,
	transform: Transform,
	motion: Motion,
	attached_to: Option<obj::Id>,
	trail_length: usize,
	pulse: Phase,
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
			trail_length: TRAIL_LENGTH,
			pulse: 0.,
			phase: 0.,
			rate: 5.,
			ttl: None,
			cluster_size: 10,
			active: true,
		}
	}
}

impl Emitter for SimpleEmitter {
	fn emit(&mut self, dt: Seconds, id_counter: &mut usize, destination: &mut HashMap<obj::Id, Particle>) -> bool {
		if self.active {
			let pulse = self.pulse;
			let phase = self.phase;
			self.phase = (self.phase + dt.get() as f32 * 2.33) % 1.;
			self.pulse = (self.pulse + dt.get() as f32 * self.rate) % 1.;
			if self.pulse < pulse {
				for i in 0..self.cluster_size {
					let alpha = consts::PI * 2. * (phase + i as f32 / self.cluster_size as f32);
					let velocity = Transform::from_angle(alpha)
						.apply_rotation(self.motion.velocity);
					let id = *id_counter;
					destination.insert(id, Particle {
						id,
						tag: 0,
						transform: Transform::new(self.transform.position, self.transform.angle + alpha),
						trail_length: self.trail_length,
						trail: VecDeque::new(),
						motion: Motion::new(velocity, self.motion.spin),
						dampening: 0.,
						friction: 0.2,
						faders: (0..MAX_FADER).map(|_| Fader::new(1.0, 0.95, 0.)).collect(),
						acceleration: -Velocity::unit_y(),
						ttl: seconds(10.),
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
	particles: HashMap<obj::Id, Particle>,
	emitters: HashMap<obj::Id, Box<Emitter>>,
	dt: Seconds,
	simulation_timer: SimulationTimer,
	simulation_clock: TimerStopwatch,
}

impl System for ParticleSystem {
	fn get_from_world(&mut self, world: &world::World) {
		if self.emitters.is_empty() {
			let emitter = SimpleEmitter::new(self.id_counter);
			self.emitters.insert(emitter.id, Box::new(emitter));
			self.id_counter += 1;
		}

		for (_id, emitter) in self.emitters.iter_mut() {
			if let Some(attached_to) = emitter.attached_to() {
				if let Some(ref agent) = world.agent(attached_to) {
					if let Some(segment) = agent.segment(0) {
						emitter.update_transform(segment.transform.clone(), segment.motion.clone());
					}
				}
			}
		}
	}

	fn update(&mut self, _: &AgentState, dt: Seconds) {
		self.dt = dt;
		self.simulation_timer.tick(dt);

		self.update_emitters();
		self.update_particles();
	}

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
				particle.faders
					.iter()
					.map(|f| f.value())
					.collect::<Vec<_>>().into_boxed_slice(),
			));
		}
	}
}

impl Default for ParticleSystem {
	fn default() -> Self {
		let simulation_timer = SimulationTimer::new();
		ParticleSystem {
			id_counter: 0,
			emitters: HashMap::new(),
			particles: HashMap::new(),
			dt: seconds(0.),
			simulation_clock: TimerStopwatch::new(&simulation_timer),
			simulation_timer,
		}
	}
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
		let trail_length = TRAIL_LENGTH;
		let expired: Vec<obj::Id> = self.particles
			.par_iter_mut().map(|(id, particle)| {
			particle.ttl -= dt;
			if particle.ttl.get() <= 0. {
				Some(*id)
			} else {
				for fader in particle.faders.iter_mut() { fader.update(dt); }
				let dt = dt.get() as f32;
				let axis_x = particle.motion.velocity.normalize();
				let axis_y = Velocity::new(-axis_x.y, axis_x.x);
				let world_acceleration = particle.acceleration.x * axis_x
					+ particle.acceleration.y * axis_y;
				if particle.trail.len() > trail_length {
					particle.trail.pop_front();
				}
				particle.trail.push_back(particle.transform.position);
				particle.motion.velocity += dt * world_acceleration;
				particle.transform.position += dt * particle.motion.velocity;
				particle.transform.angle += dt * particle.motion.spin;
				particle.motion.velocity *= 1. - (dt * particle.friction);
				particle.acceleration *= 1. - (dt * particle.dampening);
				None
			}
		}).filter_map(|i| i).collect();

		for id in expired.into_iter() {
			self.particles.remove(&id);
		}
	}
}
