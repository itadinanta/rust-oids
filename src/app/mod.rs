use app::constants::*;
use backend::messagebus::{Inbox, Message, Outbox, PubSub, ReceiveDrain, Whiteboard};
use backend::obj;
use backend::obj::*;
use backend::systems;
use backend::world;
use backend::world::agent;
use backend::world::segment;
use cgmath;
use cgmath::Matrix4;
use core::clock::*;
use core::geometry::Transform;
use core::geometry::*;
use core::math;
use core::math::Directional;
use core::math::Relative;
use core::math::Smooth;
use core::resource::ResourceLoader;
use core::util::Cycle;
use core::view::Viewport;
use core::view::WorldTransform;
use dirs;
use frontend::input;
use frontend::render;
use frontend::ui;
use getopts::Options;
use num;
use rayon::prelude::*;
use std::ffi::OsStr;
use std::fs;
use std::path;

pub use self::controller::DefaultController;
pub use self::controller::InputController;
pub use self::events::Event;
use self::events::VectorDirection;
pub use self::winit_event::WinitEventMapper;
pub use self::winit_event::WinitEventMapper as EventMapper;
use std::ffi::OsString;
use std::fmt::Debug;
use std::iter::Iterator;
use std::process;
use std::sync::Arc;
use std::sync::RwLock;

//#[cfg(feature="capture")]
mod capture;

mod controller;
mod events;
mod main;
mod paint;
mod winit_event;

pub mod constants;

pub fn run(args: &[OsString]) {
	let mut opt = Options::new();
	opt.optflag("t", "terminal", "Headless mode");
	opt.optopt("f", "fullscreen", "Fullscreen mode on monitor X", "0");
	opt.optopt("i", "initial", "Start from specific snapshot", "~/.config/rust-oids/saved_state/20180423_234300.json");
	opt.optflag("n", "new", "Ignore last snapshot, start from new population");
	opt.optopt("w", "width", "Window width", "1024");
	opt.optopt("h", "height", "Window height", "1024");
	opt.optopt("a", "audio_device", "Audio device index (portaudio)", "0");
	match opt.parse(args) {
		Ok(options) => {
			let pool_file_name = options.free.get(1).map(String::as_str).unwrap_or(DEFAULT_MINION_GENE_POOL_FILE);

			let mut world_file: Option<path::PathBuf> = options.opt_str("i").map(|s| path::Path::new(&s).to_owned());

			// we look for the last save in ~/.config/rust-oids/saved_state
			// but only if -n and -i are not specified
			let user_home = dirs::home_dir().unwrap_or_else(|| path::PathBuf::from("."));
			let config_home = user_home.join(CONFIG_DIR_HOME);
			if !options.opt_present("n") && world_file.is_none() {
				let mut max_path = None;
				if let Ok(dir) = fs::read_dir(config_home.join(CONFIG_DIR_SAVED_STATE)) {
					// get the highest file in lexicographical order
					// or the first one, if any
					for entry in dir {
						let path_name = entry.unwrap().path().to_owned();
						if path_name.extension().to_owned().and_then(OsStr::to_str) == Some("json")
							&& max_path.as_ref().map(|m| path_name > *m).unwrap_or(true)
						{
							max_path = Some(path_name.clone());
						}
					}
				}
				if max_path.is_some() {
					world_file = max_path;
					info!("Resuming simulation from snapshot: {:?}", world_file.as_ref().unwrap());
				}
			}

			if options.opt_present("t") {
				main::main_loop_headless(pool_file_name, config_home, world_file);
			} else {
				let fullscreen = options.opt_default("f", "0").and_then(|v| v.parse::<usize>().ok());
				let width = options.opt_default("w", "1024").and_then(|v| v.parse::<u32>().ok());
				let height = options.opt_default("h", "1024").and_then(|v| v.parse::<u32>().ok());
				let audio_device = options.opt_default("a", "0").and_then(|v| v.parse::<usize>().ok());

				main::main_loop(pool_file_name, config_home, world_file, fullscreen, width, height, audio_device);
			}
		}
		Err(message) => {
			eprintln!("Invalid option: {:?}", message);
			eprintln!("{}", opt.usage("rust-oids [Options]"));
			process::exit(1)
		}
	}
}

#[derive(Default)]
struct SendSystem<T>
where T: systems::System {
	ptr: Arc<RwLock<T>>,
}

impl<T> SendSystem<T>
where T: systems::System
{
	fn boxed(ptr: Arc<RwLock<T>>) -> Box<Self> { Box::new(SendSystem { ptr }) }
}

impl<T> systems::System for SendSystem<T>
where T: systems::System
{
	fn attach(&mut self, bus: &mut PubSub) { self.ptr.write().unwrap().attach(bus) }
	fn init(&mut self, world: &world::World) { self.ptr.write().unwrap().init(world) }
	fn clear(&mut self) { self.ptr.write().unwrap().clear() }
	fn register(&mut self, agent: &world::agent::Agent) { self.ptr.write().unwrap().register(agent) }
	fn unregister(&mut self, agent: &world::agent::Agent) { self.ptr.write().unwrap().unregister(agent) }

	fn step(&mut self, world: &world::World, dt: Seconds) { self.ptr.write().unwrap().step(world, dt) }
	fn apply(&self, world: &mut world::World, outbox: &Outbox) { self.ptr.read().unwrap().apply(world, outbox) }
}

// unsafe?
unsafe impl<T> Send for SendSystem<T> where T: systems::System {}

#[derive(Copy, Clone)]
pub enum SystemMode {
	Interactive,
	Batch,
}

impl Default for SystemMode {
	fn default() -> Self { SystemMode::Interactive }
}

#[derive(Default)]
pub struct Systems {
	mode: SystemMode,
	physics: Arc<RwLock<systems::PhysicsSystem>>,
	animation: Arc<RwLock<systems::AnimationSystem>>,
	game: Arc<RwLock<systems::GameSystem>>,
	ai: Arc<RwLock<systems::AiSystem>>,
	alife: Arc<RwLock<systems::AlifeSystem>>,
	particle: Arc<RwLock<systems::ParticleSystem>>,
}

impl Systems {
	fn set_mode(&mut self, mode: SystemMode) { self.mode = mode; }

	fn systems(&mut self) -> Vec<Box<(systems::System + Send)>> {
		match self.mode {
			SystemMode::Interactive => vec![
				SendSystem::boxed(self.physics.clone()),
				SendSystem::boxed(self.animation.clone()),
				SendSystem::boxed(self.particle.clone()),
				SendSystem::boxed(self.game.clone()),
				SendSystem::boxed(self.ai.clone()),
				SendSystem::boxed(self.alife.clone()),
			],
			SystemMode::Batch => vec![
				SendSystem::boxed(self.physics.clone()),
				SendSystem::boxed(self.game.clone()),
				SendSystem::boxed(self.ai.clone()),
				SendSystem::boxed(self.alife.clone()),
			],
		}
	}

	pub fn unregister(&mut self, agents: &[world::agent::Agent]) {
		if !agents.is_empty() {
			self.systems().par_iter_mut().for_each(|system| {
				for agent in agents {
					system.unregister(agent)
				}
			})
		}
	}

	pub fn register(&mut self, agents: &[world::agent::Agent]) {
		if !agents.is_empty() {
			self.systems().par_iter_mut().for_each(|system| {
				for agent in agents {
					system.register(&agent)
				}
			})
		}
	}

	fn init(&mut self, world: &world::World) {
		for system in &mut self.systems() {
			system.init(world);
		}
	}

	fn clear(&mut self) {
		for system in &mut self.systems() {
			system.clear();
		}
	}

	fn attach(&mut self, bus: &mut PubSub) {
		for system in &mut self.systems() {
			system.attach(bus);
		}
	}

	fn for_each_read(
		&mut self,
		world: &mut world::World,
		outbox: &Outbox,
		apply: &(Fn(&mut systems::System, &mut world::World, &Outbox) + Sync),
	)
	{
		self.systems().iter_mut().for_each(|r| apply(&mut (**r), world, outbox))
	}

	fn for_each_par_write(&mut self, world: &world::World, apply: &(Fn(&mut systems::System, &world::World) + Sync)) {
		self.systems().par_iter_mut().for_each(|r| apply(&mut (**r), world))
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
	pub zoom: math::ExponentialFilter<f32>,
	input_state: input::InputState,
	wall_clock: SystemTimer,
	simulations_count: usize,
	frame_count: usize,
	frame_stopwatch: TimerStopwatch,
	frame_elapsed: SimulationTimer,
	frame_smooth: math::MovingAverage<Seconds>,
	is_running: bool,
	is_paused: bool,
	is_capturing: bool,
	// interactions: Vec<Event>,
	camera: math::Inertial<f32>,
	is_camera_tracking: bool,
	lights: Cycle<Rgba>,
	backgrounds: Cycle<Rgba>,
	speed_factors: Cycle<SpeedFactor>,
	//
	world: world::World,
	bus: PubSub,
	reply_inbox: Inbox,
	alert_inbox: Inbox,
	systems: Systems,
	//
	#[allow(unused)]
	config_home: path::PathBuf,
	saved_state_dir: path::PathBuf,
	last_saved: Option<path::PathBuf>,
	//
	debug_flags: DebugFlags,
	has_ui_overlay: bool,
}

pub struct Environment {
	pub lights: Box<[render::Light]>,
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
	pub fn new<R>(
		w: u32,
		h: u32,
		scale: f32,
		config_home: path::PathBuf,
		resource_loader: &R,
		minion_gene_pool: &str,
		world_file: Option<path::PathBuf>,
	) -> Self
	where
		R: ResourceLoader<u8>,
	{
		let system_timer = SystemTimer::new();
		let mut bus = PubSub::new();
		let alert_inbox = bus.subscribe(Box::new(|e| match *e {
			Message::Alert(_) => true,
			Message::Event(_) => true,
			_ => false,
		}));
		let reply_inbox = bus.subscribe(Box::new(|e| match *e {
			Message::Event(Event::SelectMinion(_)) => true,
			_ => false,
		}));

		let mut new_world = world::World::new(resource_loader, minion_gene_pool);
		let last_saved = world_file.map(|world_file| {
			if world::persist::Serializer::load(&world_file, &mut new_world).is_err() {
				panic!(format!("Could not load {:?}", &world_file));
			}
			world_file
		});

		App {
			viewport: Viewport::rect(w, h, scale),
			zoom: math::exponential_filter(1., 1., VIEW_ZOOM_DURATION),
			input_state: input::InputState::default(),

			camera: Self::init_camera(),
			is_camera_tracking: true,
			lights: Self::init_lights(),
			backgrounds: Self::init_backgrounds(),
			speed_factors: Self::init_speed_factors(),

			world: new_world,
			bus,
			alert_inbox,
			reply_inbox,
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
			is_capturing: false,
			// savegame
			saved_state_dir: config_home.join(CONFIG_DIR_SAVED_STATE),
			config_home,
			last_saved,
			// debug
			debug_flags: DebugFlags::empty(),
			has_ui_overlay: true,
		}
	}

	fn on_app_event(&mut self, e: Event) {
		match e {
			Event::CamUp(w) => self.camera.push(math::Direction::Up, w),
			Event::CamDown(w) => self.camera.push(math::Direction::Down, w),
			Event::CamLeft(w) => self.camera.push(math::Direction::Left, w),
			Event::CamRight(w) => self.camera.push(math::Direction::Right, w),
			Event::CamReset => self.camera.reset(),

			Event::ZoomIn => {
				let target = self.zoom.last_input();
				self.zoom.input(VIEW_ZOOM_MAX.min(target * VIEW_ZOOM_MULTIPLIER))
			}
			Event::ZoomOut => {
				let target = self.zoom.last_input();
				self.zoom.input(VIEW_ZOOM_MIN.max(target / VIEW_ZOOM_MULTIPLIER))
			}
			Event::ZoomReset => self.zoom.input(1.),

			Event::VectorThrust(None, VectorDirection::None) => self.set_player_intent(segment::Intent::Idle),

			Event::VectorThrust(thrust, rotation) => {
				let pilot_rotation = match rotation {
					VectorDirection::None => segment::PilotRotation::None,
					VectorDirection::Orientation(yaw) => segment::PilotRotation::Orientation(yaw),
					VectorDirection::LookAt(target) => segment::PilotRotation::LookAt(target),
					VectorDirection::Turn(angle) => segment::PilotRotation::Turn(angle),
					VectorDirection::FromVelocity => segment::PilotRotation::FromVelocity,
				};
				self.is_camera_tracking = true;
				self.set_player_intent(segment::Intent::PilotTo(thrust.map(|v| v * THRUST_POWER), pilot_rotation));
			}
			Event::PrimaryTrigger(speed, rate) => self.primary_fire(
				BULLET_SPEED_SCALE * speed,
				BULLET_FIRE_RATE_SCALE * rate + (1. - BULLET_FIRE_RATE_SCALE),
			),

			Event::NextLight => {
				self.lights.next();
			}
			Event::PrevLight => {
				self.lights.prev();
			}
			Event::NextBackground => {
				self.backgrounds.next();
			}
			Event::PrevBackground => {
				self.backgrounds.prev();
			}
			Event::NextSpeedFactor => {
				self.speed_factors.next();
			}
			Event::PrevSpeedFactor => {
				self.speed_factors.prev();
			}
			Event::ToggleDebug => self.debug_flags.toggle(DebugFlags::DEBUG_TARGETS),
			Event::RestartFromCheckpoint => self.restart_from_checkpoint(),

			Event::AppQuit => self.quit(),
			Event::TogglePause => self.is_paused = !self.is_paused,
			Event::ToggleGui => self.has_ui_overlay = !self.has_ui_overlay,
			Event::ToggleCapture => self.is_capturing = !self.is_capturing,
			Event::SaveGenePoolToFile => self.save_gene_pool_to_file(),
			Event::SaveWorldToFile => self.save_world_to_file(),
			Event::BeginDrag(_, _) => {
				self.camera.zero();
				self.is_camera_tracking = false;
			}
			Event::Drag(start, end) => {
				self.camera.set_relative(start - end);
			}
			Event::EndDrag(start, end, vel) => {
				self.camera.set_relative(start - end);
				self.camera.velocity(vel);
			}
			Event::SelectMinion(id) => self.select_minion(id),
			Event::DeselectAll => self.deselect_all_minions(),
			Event::NewMinion(pos) => self.new_minion(pos),
			Event::RandomizeMinion(pos) => self.randomize_minion(pos),
			Event::PrimaryFire(_, _) => { /* Handled by the gameplay system */ }
			Event::Reload => { /* Handled in the main loop */ }
			Event::PickMinion(_) => { /* Handled by the physics system */ }
		}
	}

	fn init_camera() -> math::Inertial<f32> { math::Inertial::new(CAMERA_IMPULSE, CAMERA_INERTIA, CAMERA_LIMIT) }

	fn init_lights() -> Cycle<[f32; 4]> { Cycle::new(constants::AMBIENT_LIGHTS) }

	fn init_speed_factors() -> Cycle<SpeedFactor> { Cycle::new(constants::SPEED_FACTORS) }

	fn init_backgrounds() -> Cycle<[f32; 4]> { Cycle::new(constants::BACKGROUNDS) }

	fn randomize_minion(&mut self, pos: Position) { self.world.randomize_minion(pos, Motion::default()); }

	fn new_minion(&mut self, pos: Position) { self.world.new_minion(pos, Motion::default()); }

	fn primary_fire(&mut self, bullet_speed: f32, rate: SecondsValue) {
		// forwards the message to the bus
		self.bus.post(Event::PrimaryFire(bullet_speed, rate).into());
	}

	fn set_player_intent(&mut self, intent: segment::Intent) { self.world.set_player_intent(intent) }

	fn deselect_all_minions(&mut self) { self.world.for_all_agents(&mut |agent| agent.state.deselect()); }

	fn select_minion(&mut self, id: Id) {
		self.debug_flags |= DebugFlags::DEBUG_TARGETS;
		self.world.agent_mut(id).iter_mut().for_each(|a| a.state.toggle_selection());
	}

	pub fn save_gene_pool_to_file(&self) {
		match self.world.dump(&self.saved_state_dir) {
			Err(_) => error!("Failed to save gene pool"),
			Ok(path) => info!("Saved {:?}", path),
		}
	}

	pub fn save_world_to_file(&mut self) {
		let result = self.world.serialize(&self.saved_state_dir);
		match result {
			Err(_) => error!("Failed to save world state"),
			Ok(path) => {
				info!("Saved {:?}", path);
				self.set_last_saved(path);
			}
		}
	}

	fn set_last_saved(&mut self, name: path::PathBuf) { self.last_saved = Some(name) }

	pub fn interact(&mut self, e: Event) {
		self.bus.post(e.into());
		self.on_app_event(e)
	}

	pub fn has_ui_overlay(&self) -> bool { self.has_ui_overlay }

	pub fn quit(&mut self) { self.is_running = false; }

	fn restart_from_checkpoint(&mut self) {
		self.systems.clear();
		self.world.clear();
		if let Some(ref world_file) = self.last_saved {
			world::persist::Serializer::load(&world_file, &mut self.world).is_ok();
		};
		self.bus.post(world::alert::Alert::RestartFromCheckpoint.into())
	}

	pub fn is_running(&self) -> bool { self.is_running }

	pub fn is_capturing(&self) -> bool { self.is_capturing }

	pub fn on_input_event(&mut self, e: &input::Event) { self.input_state.event(e); }

	fn update_input<C>(&mut self, dt: Seconds)
	where C: InputController {
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
		let rot = Matrix4::from(cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Rad(angle)));
		let trans = Matrix4::from_translation(cgmath::Vector3::new(position.x, position.y, 0.0));

		trans * rot
	}

	pub fn init(&mut self, mode: SystemMode) {
		self.init_systems(mode);
		self.register_all();
		self.bus.post(world::alert::Alert::BeginSimulation.into());
	}

	fn register_all(&mut self) {
		// registered() drains the list, so this can be called only once per frame
		let found: Vec<agent::Agent> =
			self.world.registered().iter().filter_map(|id| self.world.agent(*id)).cloned().collect();
		self.systems.register(&found[..]);
	}

	fn cleanup_before(&mut self) {
		self.world.cleanup_before();
		self.systems.unregister(&self.world.sweep());
	}

	fn init_systems(&mut self, mode: SystemMode) {
		self.systems.set_mode(mode);
		self.systems.attach(&mut self.bus);
		self.systems.init(&self.world);
	}

	fn update_systems(&mut self, dt: Seconds) {
		self.systems.for_each_par_write(&self.world, &|s, world| s.step(&world, dt));
		self.systems.for_each_read(&mut self.world, &self.bus, &|s, mut world, outbox| s.apply(&mut world, outbox));
	}

	fn cleanup_after(&mut self) { self.register_all(); }

	fn tick(&mut self, dt: Seconds) { self.world.tick(dt); }

	pub fn receive(&mut self) {
		for event in self.reply_inbox.drain() {
			if let Message::Event(event) = event {
				self.on_app_event(event)
			}
		}
	}

	pub fn play_alerts<P, E>(&mut self, alert_player: &mut P)
	where
		P: ui::AlertPlayer<world::alert::Alert, E> + ui::AlertPlayer<Event, E>,
		E: Debug, {
		for alert in self.alert_inbox.drain() {
			match alert {
				Message::Event(ref alert) =>
					if let Err(e) = alert_player.play(alert) {
						error!("Unable to play alert {:?}", e)
					},
				Message::Alert(ref alert) =>
					if let Err(e) = alert_player.play(alert) {
						error!("Unable to play interaction {:?}", e)
					},
				_ => {}
			}
		}
	}

	pub fn update(&mut self) -> FrameUpdate { self.update_with_quantum(None) }

	pub fn update_with_quantum(&mut self, quantum_target: Option<f64>) -> FrameUpdate {
		let frame_time = self.frame_stopwatch.restart(&self.wall_clock);
		self.frame_elapsed.tick(frame_time);

		let frame_time_smooth = self.frame_smooth.smooth(frame_time);

		let player_follow =
			if self.is_camera_tracking { self.world.get_player_segment().map(|s| s.transform.position) } else { None };
		self.viewport.scale(VIEW_SCALE_BASE / self.zoom.update(frame_time_smooth.get() as f32));
		self.camera.set_inertia(CAMERA_INERTIA * self.zoom.get());
		self.camera.follow(player_follow);
		self.camera.update(frame_time_smooth);

		let target_duration = frame_time_smooth.get();

		self.update_input::<DefaultController>(frame_time_smooth);
		self.receive();
		let speed_factor = if self.is_paused { 0.0 as SpeedFactor } else { self.speed_factors.get() };
		let quantum = quantum_target.unwrap_or_else(|| num::clamp(target_duration, MIN_FRAME_LENGTH, MAX_FRAME_LENGTH));
		let (dt, rounds) = if speed_factor <= 1.0 {
			(Seconds::new(speed_factor * quantum), 1)
		} else {
			(Seconds::new(quantum), speed_factor as usize)
		};

		// dead rounds
		for _ in 0..rounds - 1 {
			self.simulate(dt);
		}

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
		self.cleanup_before();
		self.update_systems(dt);
		self.cleanup_after();
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
	fn to_world(&self, view_position: Position) -> Position { view_position + self.position() }
}
