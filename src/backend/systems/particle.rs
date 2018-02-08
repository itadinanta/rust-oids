use super::*;
use backend::obj;
use core::geometry::{Transform, Motion, Position, Velocity, Acceleration};
use app::constants::*;
use num::Zero;
use core::color::Rgba;
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

#[derive(Clone)]
struct Fader<S> {
	value: S,
	fade_rate: S,
	target: S,
}

impl<S> Default for Fader<S> where S: num::Float {
	fn default() -> Fader<S> {
		Fader {
			value: S::one(),
			fade_rate: NumCast::from(0.5).unwrap(),
			target: S::zero(),
		}
	}
}

impl<S> Fader<S> where S: num::Float {
	fn new(value: S, fade_rate: S, target: S) -> Fader<S> {
		Fader { value, fade_rate, target }
	}

	fn value(&self) -> S {
		self.value
	}

	fn with_target(self, target: S) -> Self {
		Fader {
			target,
			..self
		}
	}

	fn with_fade_rate(self, fade_rate: S) -> Self {
		Fader {
			fade_rate,
			..self
		}
	}

	fn with_value(self, value: S) -> Self {
		Fader {
			value,
			..self
		}
	}

	fn update(&mut self, dt: Seconds) -> S {
		self.value = self.value + (self.target - self.value) * self.fade_rate * NumCast::from(dt.get()).unwrap();
		self.value
	}
}

type Tag = u64;

struct ParticleBatch {
	id: obj::Id,
	tag: Tag,
	lifespan: Seconds,
	age: Seconds,
	color0: Rgba<f32>,
	color1: Rgba<f32>,
	dampening: f32,
	friction: f32,
	faders: Box<[Fader<f32>]>,
	particles: Box<[Particle]>,
}

struct Particle {
	transform: Transform,
	motion: Motion,
	acceleration: Acceleration,
	trail_length: usize,
	trail: VecDeque<Position>,
}

trait Emitter {
	fn emit(&mut self, dt: Seconds, id_counter: &mut usize, destination: &mut HashMap<obj::Id, ParticleBatch>) -> bool;
	fn attached_to(&self) -> EmitterAttachment { EmitterAttachment::None }
	fn update_transform(&mut self, _transform: Transform, _motion: Option<Motion>) {}
}

#[derive(Copy, Clone)]
enum EmitterAttachment {
	None,
	Agent(obj::Id),
	Segment(obj::Id, u8),
	Bone(obj::Id, u8, u8),
}

#[derive(Clone)]
struct SimpleEmitter {
	id: obj::Id,
	tag: Tag,
	transform: Transform,
	motion: Motion,
	attached_to: EmitterAttachment,
	trail_length: usize,
	pulse: Phase,
	phase: Phase,
	pulse_rate: f32,
	phase_rate: f32,
	color0: Rgba<f32>,
	color1: Rgba<f32>,
	dampening: f32,
	friction: f32,
	ttl: Option<Seconds>,
	cluster_size: u8,
	faders: Vec<Fader<f32>>,
	lifespan: Seconds,
	active: bool,
}

#[allow(unused)]
impl SimpleEmitter {
	fn new(id: obj::Id) -> SimpleEmitter {
		SimpleEmitter {
			id,
			tag: 0,
			transform: Transform::new(Position::zero(), 0.),
			motion: Motion::new(10. * Velocity::unit_x(), 0.),
			attached_to: EmitterAttachment::None,
			trail_length: TRAIL_LENGTH,
			pulse: 0.,
			phase: 0.,
			pulse_rate: 5.,
			phase_rate: 2.33,
			ttl: None,
			color0: COLOR_SUNSHINE,
			color1: COLOR_TRANSPARENT,
			dampening: 0.0,
			friction: 0.2,
			cluster_size: 10,
			faders: (0..MAX_FADER)
				.map(|_| Fader::default().with_fade_rate(0.95))
				.collect::<Vec<_>>(),
			lifespan: seconds(10.),
			active: true,
		}
	}

	pub fn with_attached_to(self, attached_to: EmitterAttachment) -> Self {
		SimpleEmitter {
			attached_to,
			..self
		}
	}

	pub fn with_tag(self, tag: Tag) -> Self {
		SimpleEmitter {
			tag,
			..self
		}
	}
}

impl Emitter for SimpleEmitter {
	fn emit(&mut self, dt: Seconds, id_counter: &mut usize, destination: &mut HashMap<obj::Id, ParticleBatch>) -> bool {
		if self.active {
			let pulse = self.pulse;
			let phase = self.phase;
			self.phase = (self.phase + dt.get() as f32 * self.phase_rate) % 1.;
			self.pulse = (self.pulse + dt.get() as f32 * self.pulse_rate) % 1.;
			if self.pulse < pulse {
				let particles = (0..self.cluster_size).map(|i| {
					let alpha = consts::PI * 2. * (phase + i as f32 / self.cluster_size as f32);
					let velocity = Transform::from_angle(self.transform.angle + alpha)
						.apply_rotation(self.motion.velocity);
					Particle {
						transform: Transform::new(self.transform.position, self.transform.angle + alpha),
						trail_length: self.trail_length,
						trail: VecDeque::new(),
						motion: Motion::new(velocity, self.motion.spin),
						acceleration: -Velocity::unit_y(),
					}
				}).collect::<Vec<_>>().into_boxed_slice();
				let id = *id_counter;
				*id_counter = id + 1;
				destination.insert(id,
								   ParticleBatch {
									   id,
									   tag: self.tag,
									   particles,
									   color0: self.color0,
									   color1: self.color1,
									   age: seconds(0.),
									   dampening: self.dampening,
									   friction: self.friction,
									   faders: self.faders.clone().into_boxed_slice(),
									   lifespan: self.lifespan,
								   });
			}
		}
		match self.ttl {
			None => true,
			Some(ttl) => {
				let ttl = ttl - dt;
				self.ttl = Some(ttl);
				ttl.get() > 0.
			}
		}
	}

	fn attached_to(&self) -> EmitterAttachment {
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
	particles: HashMap<obj::Id, ParticleBatch>,
	emitters: HashMap<obj::Id, Box<Emitter>>,
	dt: Seconds,
	simulation_timer: SimulationTimer,
	simulation_clock: TimerStopwatch,
}

impl System for ParticleSystem {
	fn get_from_world(&mut self, world: &world::World) {
		for source in world.emitters() {
			let emitter = match source.style {
				world::particle::EmitterStyle::Explosion { cluster_size, color } => {
					SimpleEmitter {
						id: self.next_id(),
						tag: 0,
						transform: source.transform.clone(),
						motion: Motion::new(5. * Velocity::unit_x(), 0.),
						attached_to: EmitterAttachment::None,
						trail_length: 0,
						pulse: 1.0,
						phase: 0.,
						pulse_rate: 5.,
						phase_rate: 2.33,
						color0: color,
						color1: COLOR_TRANSPARENT,
						ttl: Some(seconds(0.5)),
						cluster_size,
						dampening: 0.,
						friction: 0.2,
						faders: vec![Fader::new(1.0, 0.9, 0.0)],
						active: true,
						lifespan: seconds(3.),
					}
				}
			};
			self.emitters.insert(emitter.id, Box::new(emitter));
		}

//		if let Some(player_agent_id) = world.get_player_agent_id() {
//			if self.emitters.is_empty() {
//				let emitter = SimpleEmitter::new(self.next_id())
//					.attached_to(EmitterAttachment::Agent(player_agent_id));
//				self.emitters.insert(emitter.id, Box::new(emitter));
//			}
//		}

		let orphan: Vec<obj::Id> = self.emitters.iter_mut().map(|(id, emitter)| {
			let surviving = match emitter.attached_to() {
				EmitterAttachment::None => { Some(id) }
				EmitterAttachment::Agent(agent_id) => {
					world.agent(agent_id)
						.and_then(|agent| agent.segment(0))
						.and_then(|segment| {
							emitter.update_transform(segment.transform.clone(), segment.motion.clone());
							Some(id)
						})
				}
				EmitterAttachment::Segment(agent_id, segment_id) => {
					world.agent(agent_id)
						.and_then(|agent| agent.segment(segment_id))
						.and_then(|segment| {
							emitter.update_transform(segment.transform.clone(), segment.motion.clone());
							Some(id)
						})
				}
				EmitterAttachment::Bone(agent_id, segment_id, bone_id) => {
					world.agent(agent_id)
						.and_then(|agent| agent.segment(segment_id))
						.and_then(|segment| {
							emitter.update_transform(segment.transform.clone(), segment.motion.clone());
							Some(id)
						})
				}
			};
			match surviving {
				None => Some(id),
				Some(_) => None
			}
		})
			.filter_map(|i| i)
			.map(|i| *i)
			.collect();

		for id in &orphan {
			self.emitters.remove(id);
		}
	}

	fn update(&mut self, _: &AgentState, dt: Seconds) {
		self.dt = dt;
		self.simulation_timer.tick(dt);

		self.update_emitters();
		self.update_particles();
	}

	fn put_to_world(&self, world: &mut world::World) {
		world.clear_emitters();
		world.clear_particles();
		for (_, particle_batch) in &self.particles {
			for particle in &*particle_batch.particles {
				world.add_particle(world::particle::Particle::round(
					particle.transform.clone(),
					particle.motion.velocity.normalize(),
					particle.trail
						.iter()
						.map(|t| *t)
						.collect::<Vec<_>>().into_boxed_slice(),
					particle_batch.faders
						.iter()
						.map(|f| f.value())
						.collect::<Vec<_>>().into_boxed_slice(),
					particle_batch.color0,
					particle_batch.color1,
					particle_batch.age,
				));
			}
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
	fn next_id(&mut self) -> obj::Id {
		let counter = self.id_counter;
		self.id_counter += 1;
		self.id_counter
	}

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
			.par_iter_mut().map(|(id, particle_batch)| {
			if particle_batch.age >= particle_batch.lifespan {
				Some(*id)
			} else {
				for fader in &mut *particle_batch.faders { fader.update(dt); }
				for particle in &mut *particle_batch.particles {
					let dt = dt.get() as f32;
					// local acceleration is relative to the current velocity
					let speed = particle.motion.velocity.magnitude();
					let world_acceleration = {
						// if particle is stationary, use world frame
						if speed > 0.001 {
							let axis_x = particle.motion.velocity.normalize();
							let axis_y = Velocity::new(-axis_x.y, axis_x.x);
							particle.acceleration.x * axis_x
								+ particle.acceleration.y * axis_y
						} else {
							particle.acceleration
						}
					};
					if particle.trail.len() > trail_length {
						particle.trail.pop_front();
					}
					particle.trail.push_back(particle.transform.position);
					particle.motion.velocity += dt * world_acceleration;
					particle.transform.position += dt * particle.motion.velocity;
					particle.transform.angle += dt * particle.motion.spin;
					particle.motion.velocity *= 1. - (dt * particle_batch.friction);
					particle.acceleration *= 1. - (dt * particle_batch.dampening);
				};
				particle_batch.age += dt;
				None
			}
		}).filter_map(|i| i).collect();

		for id in expired.into_iter() {
			self.particles.remove(&id);
		}
	}
}
