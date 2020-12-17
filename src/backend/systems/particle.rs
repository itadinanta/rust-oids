use super::*;
use app::constants::*;
use backend::messagebus::{Inbox, Message, ReceiveDrain, Whiteboard};
use backend::obj;
use backend::world;
use backend::world::particle::{EmitterAttachment, EmitterStyle};
use backend::world::AgentState;
use cgmath::InnerSpace;
use core::clock::{seconds, Seconds, SimulationTimer, TimerStopwatch};
use core::color::Rgba;
use core::geometry::{Acceleration, Motion, Position, Transform, Velocity};
use num;
use num::NumCast;
use num::Zero;
use rand;
use rand::Rng;
use rayon::prelude::*;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::convert;
use std::f32::consts;
use std::iter::Iterator;

type Phase = f32;

const MAX_FADER: usize = world::particle::Fader::Count as usize;

#[derive(Clone, Copy)]
struct Fader<S>
where S: Copy {
	value: S,
	fade_rate: S,
	target: S,
}

type FaderList = [Option<Fader<f32>>; MAX_FADER];

impl<S> Default for Fader<S>
where S: num::Float
{
	fn default() -> Fader<S> { Fader { value: S::one(), fade_rate: NumCast::from(0.95).unwrap(), target: S::zero() } }
}

#[allow(unused)]
impl<S> Fader<S>
where S: num::Float
{
	fn new(value: S, fade_rate: S, target: S) -> Fader<S> { Fader { value, fade_rate, target } }

	fn flat(value: S) -> Fader<S> { Fader { value, fade_rate: S::zero(), target: value } }

	fn value(&self) -> S { self.value }

	fn with_target(self, target: S) -> Self { Fader { target, ..self } }

	fn with_fade_rate(self, fade_rate: S) -> Self { Fader { fade_rate, ..self } }

	fn with_value(self, value: S) -> Self { Fader { value, ..self } }

	fn update(&mut self, dt: Seconds) -> S {
		self.value = self.value + (self.target - self.value) * self.fade_rate * NumCast::from(dt.get()).unwrap();
		self.value
	}
}

type Tag = isize;

struct ParticleBatch {
	#[allow(unused)]
	id: obj::Id,
	tag: Tag,
	lifespan: Seconds,
	age: Seconds,
	color: (Rgba<f32>, Rgba<f32>),
	effect: (Rgba<f32>, Rgba<f32>),
	dampening: f32,
	friction: f32,
	faders: FaderList,
	trail_length: u8,
	particles: Box<[Particle]>,
}

struct Particle {
	transform: Transform,
	motion: Motion,
	acceleration: Acceleration,
	trail: VecDeque<Position>,
}

trait Emitter {
	fn emit(&mut self, dt: Seconds, id_counter: &mut usize, destination: &mut HashMap<obj::Id, ParticleBatch>) -> bool;
	fn attached_to(&self) -> EmitterAttachment { EmitterAttachment::None }
	fn update_transform(&mut self, _transform: Transform, _motion: Motion) {}
}

#[derive(Clone)]
struct SimpleEmitter {
	id: obj::Id,
	tag: Tag,
	transform: Transform,
	motion: Motion,
	frame_motion: Motion,
	acceleration: Acceleration,
	attached_to: EmitterAttachment,
	trail_length: u8,
	pulse: Phase,
	phase: Phase,
	pulse_rate: f32,
	phase_rate: f32,
	color: (Rgba<f32>, Rgba<f32>),
	effect: (Rgba<f32>, Rgba<f32>),
	dampening: f32,
	friction: f32,
	ttl: Option<Seconds>,
	cluster_size: u8,
	cluster_spread: f32,
	faders: FaderList,
	lifespan: Seconds,
	jitter: f32,
	active: bool,
}

#[allow(unused)]
impl Default for SimpleEmitter {
	fn default() -> SimpleEmitter {
		SimpleEmitter {
			id: obj::Id::default(),
			tag: Tag::default(),
			transform: Transform::default(),
			motion: Motion::default(),
			frame_motion: Motion::default(),
			acceleration: Acceleration::zero(),
			attached_to: EmitterAttachment::None,
			trail_length: 0,
			pulse: 1.0,
			phase: 0.,
			pulse_rate: 5.,
			phase_rate: 2.33,
			ttl: None,
			color: (COLOR_SUNSHINE, COLOR_TRANSPARENT),
			effect: (COLOR_WHITE, COLOR_WHITE),
			dampening: 0.,
			friction: 0.2,
			cluster_size: 10,
			cluster_spread: 1.,
			faders: [None; MAX_FADER],
			jitter: 1.,
			lifespan: seconds(10.),
			active: true,
		}
	}
}

#[allow(unused)]
impl SimpleEmitter {
	pub fn new(id: obj::Id) -> Self { SimpleEmitter::default().with_id(id) }

	pub fn with_id(self, id: obj::Id) -> Self { SimpleEmitter { id, ..self } }

	pub fn with_attached_to(self, attached_to: EmitterAttachment) -> Self { SimpleEmitter { attached_to, ..self } }

	pub fn with_tag(self, tag: Tag) -> Self { SimpleEmitter { tag, ..self } }

	pub fn with_jitter(self, jitter: f32) -> Self { SimpleEmitter { jitter, ..self } }

	pub fn with_transform(self, transform: Transform) -> Self { SimpleEmitter { transform, ..self } }

	pub fn with_motion(self, motion: Motion) -> Self { SimpleEmitter { motion, ..self } }

	pub fn with_acceleration(self, acceleration: Acceleration) -> Self { SimpleEmitter { acceleration, ..self } }

	pub fn with_trail_length(self, trail_length: u8) -> Self { SimpleEmitter { trail_length, ..self } }

	pub fn with_pulse(self, pulse: f32, pulse_rate: f32) -> Self { SimpleEmitter { pulse, pulse_rate, ..self } }

	pub fn with_phase(self, phase: f32, phase_rate: f32) -> Self { SimpleEmitter { phase, phase_rate, ..self } }

	pub fn with_ttl(self, ttl: Option<Seconds>) -> Self { SimpleEmitter { ttl, ..self } }

	pub fn with_color(self, color0: Rgba<f32>, color1: Rgba<f32>) -> Self {
		SimpleEmitter { color: (color0, color1), ..self }
	}

	pub fn with_effect(self, effect0: Rgba<f32>, effect1: Rgba<f32>) -> Self {
		SimpleEmitter { effect: (effect0, effect1), ..self }
	}

	pub fn with_friction(self, friction: f32, dampening: f32) -> Self { SimpleEmitter { friction, dampening, ..self } }

	pub fn with_1_fader(self, fader_color: Fader<f32>) -> Self {
		self.with_faders([Some(fader_color), None, None, None])
	}

	pub fn with_2_faders(self, fader_color: Fader<f32>, fader_scale: Fader<f32>) -> Self {
		self.with_faders([Some(fader_color), Some(fader_scale), None, None])
	}

	pub fn with_3_faders(self, fader_color: Fader<f32>, fader_scale: Fader<f32>, fader_effect: Fader<f32>) -> Self {
		self.with_faders([Some(fader_color), Some(fader_scale), Some(fader_effect), None])
	}

	pub fn with_4_faders(
		self,
		fader_color: Fader<f32>,
		fader_scale: Fader<f32>,
		fader_effect: Fader<f32>,
		fader_frequency: Fader<f32>,
	) -> Self {
		self.with_faders([Some(fader_color), Some(fader_scale), Some(fader_effect), Some(fader_frequency)])
	}

	pub fn with_fader(self, index: world::particle::Fader, value: Option<Fader<f32>>) -> Self {
		let mut faders = self.faders;
		faders[index as usize] = value;
		SimpleEmitter { faders, ..self }
	}

	pub fn with_faders(self, faders: FaderList) -> Self { SimpleEmitter { faders, ..self } }

	pub fn with_lifespan(self, lifespan: Seconds) -> Self { SimpleEmitter { lifespan, ..self } }

	pub fn with_cluster_size(self, cluster_size: u8) -> Self { SimpleEmitter { cluster_size, ..self } }
	pub fn with_cluster_spread(self, cluster_spread: f32) -> Self { SimpleEmitter { cluster_spread, ..self } }
}

impl Emitter for SimpleEmitter {
	fn emit(&mut self, dt: Seconds, id_counter: &mut usize, destination: &mut HashMap<obj::Id, ParticleBatch>) -> bool {
		let mut rng = rand::thread_rng();
		let jitter_value = self.jitter;
		let mut jitter = move |w| (rng.next_f32() * 2. * w - w) * jitter_value + 1.;
		if self.active {
			let pulse = self.pulse;
			let phase = self.phase;
			self.phase = (self.phase + dt * self.phase_rate * jitter(0.1)) % 1.;
			self.pulse = (self.pulse + dt * self.pulse_rate * jitter(0.1)) % 1.;
			if self.pulse < pulse {
				let particles = (0..self.cluster_size)
					.map(|i| {
						let cluster_size = <f32 as convert::From<_>>::from(self.cluster_size);
						let spread = self.cluster_spread / cluster_size
							* (<f32 as convert::From<_>>::from(i) - (cluster_size - 1.) / 2.);
						let alpha = consts::PI * 2. * (phase + spread);
						let velocity = Transform::from_angle(self.transform.angle + alpha + jitter(0.1) - 1.)
							.apply_rotation(self.motion.velocity * jitter(0.1));
						let frame_velocity = self.frame_motion.velocity;
						Particle {
							transform: Transform::new(
								self.transform.position,
								self.transform.angle + alpha + jitter(0.1) - 1.,
							),
							trail: VecDeque::new(),
							motion: Motion::new(velocity + frame_velocity, self.motion.spin + jitter(0.1) - 1.),
							acceleration: self.acceleration * jitter(0.5),
						}
					})
					.collect::<Vec<_>>()
					.into_boxed_slice();
				let id = *id_counter;
				*id_counter = id + 1;
				destination.insert(id, ParticleBatch {
					id,
					tag: self.tag,
					particles,
					color: self.color,
					effect: self.effect,
					age: seconds(0.),
					trail_length: self.trail_length,
					dampening: self.dampening,
					friction: self.friction,
					faders: self.faders,
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

	fn attached_to(&self) -> EmitterAttachment { self.attached_to }

	fn update_transform(&mut self, transform: Transform, motion: Motion) {
		self.transform = transform;
		self.frame_motion = motion;
	}
}

#[allow(unused)]
pub struct ParticleSystem {
	id_counter: usize,
	inbox: Option<Inbox>,
	particles: HashMap<obj::Id, ParticleBatch>,
	emitters: HashMap<obj::Id, Box<dyn Emitter>>,
	dt: Seconds,
	simulation_timer: SimulationTimer,
	simulation_clock: TimerStopwatch,
}

impl System for ParticleSystem {
	fn attach(&mut self, bus: &mut PubSub) {
		self.inbox = Some(bus.subscribe(Box::new(|m| if let Message::NewEmitter(_) = *m { true } else { false })));
	}

	fn clear(&mut self) {
		self.particles.clear();
		self.emitters.clear();
	}

	fn import(&mut self, world: &world::World) {
		let mut emitters = Vec::new();
		if let Some(ref inbox) = self.inbox {
			for message in inbox.drain() {
				if let Message::NewEmitter(emitter) = message {
					emitters.push(emitter);
				}
			}
		}
		for source in emitters {
			let emitter = match source.style {
				EmitterStyle::Explosion { cluster_size, color } => SimpleEmitter::new(self.next_id())
					.with_transform(source.transform.clone())
					.with_motion(Motion::new(10. * Velocity::unit_x(), 0.))
					.with_color(color, COLOR_TRANSPARENT)
					.with_ttl(Some(seconds(0.5)))
					.with_cluster_size(cluster_size)
					.with_cluster_spread(0.5)
					.with_2_faders(Fader::new(0.0, 2.0, 1.0), Fader::new(1.0, 0.7, 0.1))
					.with_lifespan(seconds(3.0)),
				EmitterStyle::Ping { color } => SimpleEmitter::new(self.next_id())
					.with_attached_to(source.attached_to)
					.with_transform(source.transform.clone())
					.with_color(color, COLOR_TRANSPARENT)
					.with_ttl(Some(seconds(0.33)))
					.with_4_faders(
						Fader::new(0.0, 4.0, 1.0),
						Fader::new(0.5, 0.7, 5.0),
						Fader::flat(1.),
						Fader::new(10., 1., 5.),
					)
					.with_cluster_size(1)
					.with_lifespan(seconds(3.0)),
				EmitterStyle::Sparkle { cluster_size, color } => SimpleEmitter::new(self.next_id())
					.with_jitter(0.)
					.with_transform(source.transform.clone())
					.with_motion(Motion::new(8. * Velocity::unit_x(), 0.))
					.with_effect(COLOR_WHITE, [1., 1., 1., 16.0])
					.with_color(color, COLOR_TRANSPARENT)
					.with_ttl(Some(seconds(0.30)))
					.with_pulse(1.0, 10.)
					.with_phase(0.0, 1. / <f32 as convert::From<_>>::from(cluster_size))
					.with_3_faders(Fader::new(0.0, 1.2, 1.0), Fader::default(), Fader::default())
					.with_cluster_size(cluster_size)
					.with_trail_length(5)
					.with_acceleration(-5. * Velocity::unit_y())
					.with_lifespan(seconds(3.0)),
			};
			self.emitters.insert(emitter.id, Box::new(emitter));
		}

		// Player trail
		if let Some(player_agent_id) = world.get_player_agent_id() {
			if self.emitters.is_empty() {
				let emitter = SimpleEmitter::new(self.next_id())
					.with_attached_to(EmitterAttachment::Vertex(player_agent_id, 0, 10))
					.with_motion(Motion::new(10. * Velocity::unit_x(), 0.))
					.with_color(COLOR_SUNSHINE, COLOR_TRANSPARENT)
					.with_effect(COLOR_WHITE, [1., 1., 1., 16.0])
					.with_pulse(0., 60. / 5.)
					.with_phase(0., 0.)
					.with_jitter(3.)
					.with_4_faders(
						Fader::new(0.0, 4.0, 1.0),
						Fader::new(1.0, 1.1, 0.1),
						Fader::default(),
						Fader::new(1.0, 4.0, 0.0),
					)
					.with_cluster_size(3)
					.with_cluster_spread(0.15)
					.with_lifespan(seconds(3.0));
				self.emitters.insert(emitter.id, Box::new(emitter));
			}
		}

		let orphan: Vec<obj::Id> = self
			.emitters
			.iter_mut()
			.map(|(id, emitter)| {
				let surviving = match emitter.attached_to() {
					EmitterAttachment::None => Some(id),
					EmitterAttachment::Agent(agent_id) =>
						world.agent(agent_id).and_then(|agent| agent.segment(0)).map(|segment| {
							emitter.update_transform(segment.transform.clone(), segment.motion.clone());
							id
						}),
					EmitterAttachment::Segment(agent_id, segment_id) =>
						world.agent(agent_id).and_then(|agent| agent.segment(segment_id)).map(|segment| {
							emitter.update_transform(segment.transform.clone(), segment.motion.clone());
							id
						}),
					EmitterAttachment::Vertex(agent_id, segment_id, bone_id) =>
						world.agent(agent_id).and_then(|agent| agent.segment(segment_id)).map(|segment| {
							let vertex = segment.growing_scaled_vertex(bone_id as usize);
							let vertex_angle = f32::atan2(vertex.y, vertex.x);
							let transform =
								Transform::new(segment.transform.apply(vertex), segment.transform.angle + vertex_angle);
							emitter.update_transform(transform, segment.motion.clone());
							id
						}),
				};
				match surviving {
					None => Some(id),
					Some(_) => None,
				}
			})
			.filter_map(|i| i)
			.cloned()
			.collect();

		for id in &orphan {
			self.emitters.remove(id);
		}
	}

	fn update(&mut self, _: &dyn AgentState, dt: Seconds) {
		self.dt = dt;
		self.simulation_timer.tick(dt);

		self.update_emitters();
		self.update_particles();
	}

	fn export(&self, world: &mut world::World, _outbox: &dyn Outbox) {
		for particle_batch in self.particles.values() {
			for particle in &*particle_batch.particles {
				let mut faders = [1.; MAX_FADER];
				for (src, dest) in particle_batch.faders.iter().zip(faders.iter_mut()) {
					*dest = src.map(|f| f.value()).unwrap_or(1.)
				}
				world.add_particle(world::particle::Particle::new(
					particle.transform.clone(),
					particle.motion.velocity.normalize(),
					particle_batch.tag,
					particle.trail.iter().cloned().collect::<Vec<_>>().into_boxed_slice(),
					faders,
					particle_batch.color,
					particle_batch.effect,
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
			inbox: None,
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
		let expired: Vec<obj::Id> = self
			.particles
			.par_iter_mut()
			.filter_map(|(id, particle_batch)| {
				if particle_batch.age >= particle_batch.lifespan {
					Some(*id)
				} else {
					for f in &mut particle_batch.faders {
						if let Some(ref mut v) = *f {
							v.update(dt);
						}
					}
					for particle in &mut *particle_batch.particles {
						let dt = dt.get() as f32;
						// local acceleration is relative to the current velocity
						let speed = particle.motion.velocity.magnitude();
						let world_acceleration = {
							// if particle is stationary, use world frame
							if speed > 0.001 {
								let axis_x = particle.motion.velocity.normalize();
								let axis_y = Velocity::new(-axis_x.y, axis_x.x);
								particle.acceleration.x * axis_x + particle.acceleration.y * axis_y
							} else {
								particle.acceleration
							}
						};
						while particle.trail.len() > particle_batch.trail_length as usize {
							particle.trail.pop_back();
						}
						particle.trail.push_front(particle.transform.position);
						particle.motion.velocity += dt * world_acceleration;
						particle.transform.position += dt * particle.motion.velocity;
						particle.transform.angle += dt * particle.motion.spin;
						particle.motion.velocity *= 1. - (dt * particle_batch.friction);
						particle.acceleration *= 1. - (dt * particle_batch.dampening);
					}
					particle_batch.age += dt;
					None
				}
			})
			.collect();

		for id in expired {
			self.particles.remove(&id);
		}
	}
}
