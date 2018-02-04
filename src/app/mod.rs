mod main;
mod winit_event;
mod controller;
mod events;
mod paint;

pub mod constants;

use std::process;
use std::rc::Rc;
use std::cell::RefCell;
use std::fmt::Debug;

use core::util::Cycle;
use core::geometry::*;
use core::geometry::Transform;
use core::clock::*;
use core::math;
use core::math::Directional;
use core::math::Relative;
use core::math::Smooth;

use core::resource::ResourceLoader;

use backend::obj;
use backend::obj::*;
use backend::world;
use backend::world::segment;
use backend::world::agent;
use backend::systems;
use backend::systems::System;

use core::view::Viewport;
use core::view::WorldTransform;

use frontend::input;
use frontend::ui;
use getopts::Options;
use std::ffi::OsString;

use app::constants::*;

use num;
use cgmath;
use cgmath::{Matrix4, SquareMatrix};
use std::iter::Iterator;
use rayon::prelude::*;

pub use self::winit_event::WinitEventMapper;
pub use self::winit_event::WinitEventMapper as EventMapper;
pub use self::controller::InputController;
pub use self::controller::DefaultController;
pub use self::events::Event;
use self::events::VectorDirection;

pub fn run(args: &[OsString]) {
	let mut opt = Options::new();
	opt.optflag("t", "terminal", "Headless mode");
	opt.optopt("f", "fullscreen", "Fullscreen mode on monitor X", "0");
	opt.optopt("w", "width", "Window width", "1024");
	opt.optopt("h", "height", "Window height", "1024");
	match opt.parse(args) {
		Ok(options) => {
			let pool_file_name = options.free.get(1).map(|n| n.as_str()).unwrap_or(
				"minion_gene_pool.csv",
			);
			if options.opt_present("t") {
				main::main_loop_headless(pool_file_name);
			} else {
				let fullscreen = options.opt_default("f", "0").and_then(|v| v.parse::<usize>().ok());
				let width = options.opt_default("w", "1024").and_then(|v| v.parse::<u32>().ok());
				let height = options.opt_default("h", "1024").and_then(|v| v.parse::<u32>().ok());

				main::main_loop(pool_file_name, fullscreen, width, height);
			}
		}
		Err(message) => {
			eprintln!("Invalid option: {:?}", message);
			eprintln!("{}", opt.usage("rust-oids [Options]"));
			process::exit(1)
		}
	}
}

use std::sync::Arc;
use std::sync::RwLock;

#[derive(Default)]
struct SendSystem<T> where T: systems::System {
	ptr: Arc<RwLock<T>>
}

impl<T> SendSystem<T> where T: systems::System {
	fn boxed(ptr: Arc<RwLock<T>>) -> Box<Self> {
		Box::new(SendSystem { ptr })
	}
}

impl<T> systems::Updateable for SendSystem<T> where T: systems::System {
	fn update(&mut self, world_state: &world::WorldState, dt: Seconds) { self.ptr.write().unwrap().update(world_state, dt) }
}

impl<T> systems::System for SendSystem<T> where T: systems::System {
	fn init(&mut self, world: &world::World) { self.ptr.write().unwrap().init(world) }
	fn register(&mut self, agent: &world::agent::Agent) { self.ptr.write().unwrap().register(agent) }
	fn unregister(&mut self, agent: &world::agent::Agent) { self.ptr.write().unwrap().unregister(agent) }
	fn get_from_world(&mut self, world: &world::World) { self.ptr.write().unwrap().get_from_world(world) }
	fn put_to_world(&self, world: &mut world::World) { self.ptr.read().unwrap().put_to_world(world) }
	fn update_world(&mut self, world: &mut world::World, dt: Seconds) { self.ptr.write().unwrap().update_world(world, dt) }
}

// unsafe?
unsafe impl<T> Send for SendSystem<T> where T: systems::System {}

pub struct Systems {
	physics: Arc<RwLock<systems::PhysicsSystem>>,
	animation: Arc<RwLock<systems::AnimationSystem>>,
	game: Arc<RwLock<systems::GameSystem>>,
	ai: Arc<RwLock<systems::AiSystem>>,
	alife: Arc<RwLock<systems::AlifeSystem>>,
	particle: Arc<RwLock<systems::ParticleSystem>>,
}

impl<'l> Default for Systems {
	fn default() -> Self {
		let physics = Arc::new(RwLock::new(systems::PhysicsSystem::default()));
		let animation = Arc::new(RwLock::new(systems::AnimationSystem::default()));
		let game = Arc::new(RwLock::new(systems::GameSystem::default()));
		let ai = Arc::new(RwLock::new(systems::AiSystem::default()));
		let alife = Arc::new(RwLock::new(systems::AlifeSystem::default()));
		let particle = Arc::new(RwLock::new(systems::ParticleSystem::default()));
		Systems {
			physics,
			animation,
			game,
			ai,
			alife,
			particle,
		}
	}
}

use std::borrow::BorrowMut;

impl Systems {
	fn systems(&mut self) -> Vec<Box<(systems::System + Send)>> {
		vec![
			SendSystem::boxed(self.physics.clone()),
			SendSystem::boxed(self.animation.clone()),
			SendSystem::boxed(self.particle.clone()),
			SendSystem::boxed(self.game.clone()),
			SendSystem::boxed(self.ai.clone()),
			SendSystem::boxed(self.alife.clone()),
			SendSystem::boxed(self.physics.clone()),
		]
	}

	pub fn unregister(&mut self, agent: &world::agent::Agent) {
		self.systems().par_iter_mut().for_each(|system| system.unregister(agent))
	}

	fn for_each(&mut self, apply: &(Fn(&mut systems::System) + Sync)) {
		self.systems().par_iter_mut().for_each(
			|r| apply(&mut (**r))
		)
	}

	fn from_world(&mut self, world: &world::World, apply: &(Fn(&mut systems::System, &world::World) + Sync)) {
		//for r in self.systems().as_mut_slice() {
		self.systems().par_iter_mut().for_each(
			|r| apply(&mut (**r), &world)
		)
	}

	fn to_world(&mut self, mut world: &mut world::World, apply: &(Fn(&mut systems::System, &mut world::World) + Sync)) {
		self.systems().iter_mut().for_each(
			|r| apply(&mut (**r), &mut world)
		)
	}
}

bitflags! {
	pub struct DebugFlags: u32 {
		const DEBUG_TARGETS = 0x1;
	}
}

pub type SpeedFactor = f64;

pub struct App {
	pub viewport: Viewport,
	input_state: input::InputState,
	wall_clock: SystemTimer,
	simulations_count: usize,
	frame_count: usize,
	frame_stopwatch: TimerStopwatch,
	frame_elapsed: SimulationTimer,
	frame_smooth: math::MovingAverage<Seconds>,
	is_running: bool,
	is_paused: bool,
	interactions: Vec<Event>,
	//
	camera: math::Inertial<f32>,
	lights: Cycle<Rgba>,
	backgrounds: Cycle<Rgba>,
	speed_factors: Cycle<SpeedFactor>,
	//
	world: world::World,
	systems: Systems,
	//
	debug_flags: DebugFlags,
	has_ui_overlay: bool,
}

pub struct Environment {
	pub light_color: Rgba,
	pub light_positions: Box<[Position]>,
	pub background_color: Rgba,
}

#[derive(Clone, Debug)]
pub struct SimulationUpdate {
	pub timestamp: Seconds,
	pub dt: Seconds,
	pub count: usize,
	pub elapsed: Seconds,
	pub population: usize,
	pub extinctions: usize,
}

#[derive(Clone, Debug)]
pub struct FrameUpdate {
	pub timestamp: Seconds,
	pub dt: Seconds,
	pub speed_factor: f32,
	pub count: usize,
	pub elapsed: Seconds,
	pub duration_smooth: Seconds,
	pub fps: f32,
	pub simulation: SimulationUpdate,
}

impl App {
	pub fn new<R>(w: u32, h: u32, scale: f32, resource_loader: &R, minion_gene_pool: &str) -> Self
		where
			R: ResourceLoader<u8>, {
		let system_timer = SystemTimer::new();
		App {
			viewport: Viewport::rect(w, h, scale),
			input_state: input::InputState::default(),
			interactions: Vec::new(),

			camera: Self::init_camera(),
			lights: Self::init_lights(),
			backgrounds: Self::init_backgrounds(),
			speed_factors: Self::init_speed_factors(),

			world: world::World::new(resource_loader, minion_gene_pool),
			// subsystems
			systems: Systems::default(),
			// runtime and timing
			simulations_count: 0usize,
			frame_count: 0usize,
			frame_elapsed: SimulationTimer::new(),
			frame_stopwatch: TimerStopwatch::new(&system_timer),
			wall_clock: system_timer,
			frame_smooth: math::MovingAverage::new(FRAME_SMOOTH_COUNT),
			is_running: true,
			is_paused: false,
			// debug
			debug_flags: DebugFlags::empty(),
			has_ui_overlay: true,
		}
	}

	fn init_camera() -> math::Inertial<f32> {
		math::Inertial::new(5.0, 1.0, 0.5)
	}

	fn init_lights() -> Cycle<[f32; 4]> {
		Cycle::new(constants::AMBIENT_LIGHTS)
	}

	fn init_speed_factors() -> Cycle<SpeedFactor> {
		Cycle::new(constants::SPEED_FACTORS)
	}

	fn init_backgrounds() -> Cycle<[f32; 4]> {
		Cycle::new(constants::BACKGROUNDS)
	}

	pub fn pick_minion(&self, pos: Position) -> Option<Id> {
		self.systems.physics.read().unwrap().pick(pos)
	}

	fn randomize_minion(&mut self, pos: Position) {
		self.world.randomize_minion(pos, None);
	}

	fn new_minion(&mut self, pos: Position) {
		self.world.new_minion(pos, None);
	}

	fn primary_fire(&mut self, bullet_speed: f32, rate: SecondsValue) {
		self.systems.game.write().unwrap().primary_fire(bullet_speed, rate)
	}

	pub fn set_player_intent(&mut self, intent: segment::Intent) {
		self.world.set_player_intent(intent)
	}

	fn deselect_all(&mut self) {
		self.world.for_all_agents(
			&mut |agent| agent.state.deselect(),
		);
	}

	fn select_minion(&mut self, id: Id) {
		self.debug_flags |= DebugFlags::DEBUG_TARGETS;
		self.world.agent_mut(id).map(|a| a.state.toggle_selection());
	}

	pub fn dump_to_file(&self) {
		match self.world.dump() {
			Err(_) => error!("Failed to dump log"),
			Ok(name) => info!("Saved {}", name),
		}
	}

	pub fn interact(&mut self, e: Event) {
		self.interactions.push(e);
		self.on_app_event(e)
	}

	fn on_app_event(&mut self, e: Event) {
		match e {
			Event::CamUp(w) => self.camera.push(math::Direction::Up, w),
			Event::CamDown(w) => self.camera.push(math::Direction::Down, w),
			Event::CamLeft(w) => self.camera.push(math::Direction::Left, w),
			Event::CamRight(w) => self.camera.push(math::Direction::Right, w),

			Event::VectorThrust(None, VectorDirection::None) => {
				self.world.set_player_intent(segment::Intent::Idle);
			}
			Event::VectorThrust(thrust, rotation) => {
				let pilot_rotation = match rotation {
					VectorDirection::None => segment::PilotRotation::None,
					VectorDirection::Orientation(yaw) => segment::PilotRotation::Orientation(yaw),
					VectorDirection::LookAt(target) => segment::PilotRotation::LookAt(target),
					VectorDirection::Turn(angle) => segment::PilotRotation::Turn(angle),
				};
				self.set_player_intent(segment::Intent::PilotTo(thrust.map(|v| v * THRUST_POWER), pilot_rotation));
			}
			Event::PrimaryFire(speed, rate) => {
				self.primary_fire(BULLET_SPEED_SCALE * speed,
								  BULLET_FIRE_RATE_SCALE * rate + (1. - BULLET_FIRE_RATE_SCALE));
			}
			Event::CamReset => { self.camera.reset(); }
			Event::NextLight => { self.lights.next(); }
			Event::PrevLight => { self.lights.prev(); }
			Event::NextBackground => { self.backgrounds.next(); }
			Event::PrevBackground => { self.backgrounds.prev(); }
			Event::NextSpeedFactor => { self.speed_factors.next(); }
			Event::PrevSpeedFactor => { self.speed_factors.prev(); }
			Event::ToggleDebug => self.debug_flags.toggle(DebugFlags::DEBUG_TARGETS),
			Event::Reload => {}

			Event::AppQuit => self.quit(),
			Event::TogglePause => self.is_paused = !self.is_paused,
			Event::ToggleGui => self.has_ui_overlay = !self.has_ui_overlay,
			Event::DumpToFile => self.dump_to_file(),
			Event::BeginDrag(_, _) => { self.camera.zero(); }
			Event::Drag(start, end) => { self.camera.set_relative(start - end); }
			Event::EndDrag(start, end, vel) => {
				self.camera.set_relative(start - end);
				self.camera.velocity(vel);
			}
			Event::PickMinion(position) => {
				self.pick_minion(position).map(|id|
					self.select_minion(id));
			}
			Event::DeselectAll => self.deselect_all(),
			Event::NewMinion(pos) => self.new_minion(pos),
			Event::RandomizeMinion(pos) => self.randomize_minion(pos),
		}
	}

	pub fn has_ui_overlay(&self) -> bool {
		self.has_ui_overlay
	}

	pub fn quit(&mut self) {
		self.is_running = false;
	}

	pub fn is_running(&self) -> bool {
		self.is_running
	}

	pub fn on_input_event(&mut self, e: &input::Event) {
		self.input_state.event(e);
	}

	fn update_input<C>(&mut self, dt: Seconds) where C: InputController {
		self.input_state.pre_update(&self.viewport);

		for e in C::update(&self.input_state, &self.viewport, &self.camera, dt) {
			self.interact(e)
		}
		self.input_state.post_update();
	}

	pub fn on_resize(&mut self, width: u32, height: u32) {
		self.viewport = Viewport::rect(width, height, self.viewport.scale);
	}

	fn from_transform(transform: &Transform) -> Matrix4<f32> {
		use cgmath::Rotation3;
		let position = transform.position;
		let angle = transform.angle;
		let rot = Matrix4::from(cgmath::Quaternion::from_axis_angle(
			cgmath::Vector3::unit_z(),
			cgmath::Rad(angle),
		));
		let trans = Matrix4::from_translation(cgmath::Vector3::new(position.x, position.y, 0.0));

		trans * rot
	}

	fn from_position(position: &Position) -> Matrix4<f32> {
		Matrix4::from_translation(cgmath::Vector3::new(position.x, position.y, 0.0))
	}
}

impl App {
	pub fn init(&mut self) {
		use backend::world::AlertReceiver;
		self.init_systems();
		self.world.alert(world::alert::Alert::BeginSimulation);
	}

	fn register_all(&mut self) {
		for id in self.world.registered().into_iter() {
			if let Some(found) = self.world.agent_mut(*id) {
				self.systems.physics.write().unwrap().register(found);
			}
		}
	}

	fn init_systems(&mut self) {
		self.systems.from_world(
			&self.world,
			&|s, world| s.init(&world),
		);
	}

	fn cleanup(&mut self) {
		let freed = self.world.sweep();
		for freed_agent in freed.iter() {
			self.systems.unregister(freed_agent);
		}
	}

	fn tick(&mut self, dt: Seconds) {
		self.world.tick(dt);
	}

	fn update_systems(&mut self, dt: Seconds) {
		self.systems.to_world(&mut self.world, &|s, mut world| {
			s.update_world(&mut world, dt)
		});
	}

	pub fn play_alerts<P, E>(&mut self, player: &mut P) where P: ui::AlertPlayer<world::alert::AlertEvent, E>, E: Debug {
		for alert in self.world.consume_alerts().into_iter() {
			match player.play(alert) {
				Err(e) => error!("Unable to play alert {:?}", e),
				Ok(_) => ()
			}
		}
	}

	pub fn play_interactions<P, E>(&mut self, player: &mut P) where P: ui::AlertPlayer<Event, E>, E: Debug {
		for alert in self.interactions.drain(..) {
			match player.play(&alert) {
				Err(e) => error!("Unable to play interaction {:?}", e),
				Ok(_) => ()
			}
		}
	}

	pub fn update(&mut self) -> FrameUpdate {
		let frame_time = self.frame_stopwatch.restart(&self.wall_clock);
		self.frame_elapsed.tick(frame_time);

		let frame_time_smooth = self.frame_smooth.smooth(frame_time);
		self.camera.follow(self.world.get_player_world_position());
		self.camera.update(frame_time_smooth);

		let target_duration = frame_time_smooth.get();

		self.update_input::<DefaultController>(frame_time_smooth);

		let speed_factor = if self.is_paused { 0.0 as SpeedFactor } else { self.speed_factors.get() };
		let quantum = num::clamp(target_duration, MIN_FRAME_LENGTH, MAX_FRAME_LENGTH);
		let (dt, rounds) = if speed_factor <= 1.0 {
			(Seconds::new(speed_factor * quantum), 1)
		} else {
			(Seconds::new(quantum), speed_factor as usize)
		};

		if rounds > 1 {
			// dead rounds
			for _ in 0..rounds - 1 {
				self.simulate(dt);
			}
		};
		let simulation_update = self.simulate(dt);
		self.frame_count += 1;

		FrameUpdate {
			timestamp: self.wall_clock.seconds(),
			dt: frame_time,
			speed_factor: speed_factor as f32,
			count: self.frame_count,
			elapsed: self.frame_elapsed.seconds(),
			duration_smooth: frame_time_smooth,
			fps: 1. / target_duration as f32,
			simulation: simulation_update,
		}
	}

	pub fn simulate(&mut self, dt: Seconds) -> SimulationUpdate {
		self.cleanup();
		self.update_systems(dt);
		self.register_all();
		self.tick(dt);

		self.simulations_count += 1;

		SimulationUpdate {
			timestamp: self.wall_clock.seconds(),
			dt,
			count: self.simulations_count,
			elapsed: self.world.seconds(),
			population: self.world.agents(agent::AgentType::Minion).len(),
			extinctions: self.world.extinctions(),
		}
	}
}

impl WorldTransform for math::Inertial<f32> {
	fn to_world(&self, view_position: Position) -> Position {
		view_position + self.position()
	}
}

