use app::constants::*;
use backend::obj;
use backend::obj::*;
use backend::systems;
use backend::systems::messagebus::{Inbox, ReceiveDrain, Outbox, PubSub, Whiteboard, Message};
use backend::world;
use backend::world::Alert;
use backend::world::AlertReceiver;
use backend::world::agent;
use backend::world::segment;
use cgmath;
use cgmath::Matrix4;
use core::clock::*;
use core::geometry::*;
use core::geometry::Transform;
use core::math;
use core::math::Directional;
use core::math::Relative;
use core::math::Smooth;
use core::resource::ResourceLoader;
use core::util::Cycle;
use core::view::Viewport;
use core::view::WorldTransform;
use frontend::input;
use frontend::ui;
use getopts::Options;
use num;
use rayon::prelude::*;

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

mod main;
mod winit_event;
mod controller;
mod events;
mod paint;

pub mod constants;

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

#[derive(Default)]
struct SendSystem<T> where T: systems::System {
	ptr: Arc<RwLock<T>>
}

impl<T> SendSystem<T> where T: systems::System {
	fn boxed(ptr: Arc<RwLock<T>>) -> Box<Self> {
		Box::new(SendSystem { ptr })
	}
}

impl<T> systems::System for SendSystem<T> where T: systems::System {
	fn attach(&mut self, bus: &mut PubSub) { self.ptr.write().unwrap().attach(bus) }
	fn init(&mut self, world: &world::World) { self.ptr.write().unwrap().init(world) }
	fn register(&mut self, agent: &world::agent::Agent) { self.ptr.write().unwrap().register(agent) }
	fn unregister(&mut self, agent: &world::agent::Agent) { self.ptr.write().unwrap().unregister(agent) }

	fn step(&mut self, world: &world::World, dt: Seconds) { self.ptr.write().unwrap().step(world, dt) }
	fn apply(&self, world: &mut world::World, outbox: &Outbox) { self.ptr.read().unwrap().apply(world, outbox) }
}

// unsafe?
unsafe impl<T> Send for SendSystem<T> where T: systems::System {}

#[derive(Default)]
pub struct Systems {
	physics: Arc<RwLock<systems::PhysicsSystem>>,
	animation: Arc<RwLock<systems::AnimationSystem>>,
	game: Arc<RwLock<systems::GameSystem>>,
	ai: Arc<RwLock<systems::AiSystem>>,
	alife: Arc<RwLock<systems::AlifeSystem>>,
	particle: Arc<RwLock<systems::ParticleSystem>>,
}

impl Systems {
	fn systems(&mut self) -> Vec<Box<(systems::System + Send)>> {
		vec![
			SendSystem::boxed(self.physics.clone()),
			SendSystem::boxed(self.animation.clone()),
			SendSystem::boxed(self.particle.clone()),
			SendSystem::boxed(self.game.clone()),
			SendSystem::boxed(self.ai.clone()),
			SendSystem::boxed(self.alife.clone()),
		]
	}

	pub fn unregister(&mut self, agents: &[world::agent::Agent]) {
		if !agents.is_empty() {
			self.systems().par_iter_mut().for_each(
				|system| {
					for agent in agents {
						system.unregister(agent)
					}
				}
			)
		}
	}

	pub fn register(&mut self, agents: &[world::agent::Agent]) {
		if !agents.is_empty() {
			self.systems().par_iter_mut().for_each(
				|system| for agent in agents {
					system.register(&agent)
				}
			)
		}
	}

	fn init(&mut self, world: &world::World) {
		for system in self.systems().iter_mut() {
			system.init(world);
		}
	}

	fn attach(&mut self, bus: &mut PubSub) {
		for system in self.systems().iter_mut() {
			system.attach(bus);
		}
	}

	fn for_each_read(&mut self, world: &mut world::World, outbox: &Outbox, apply: &(Fn(&mut systems::System, &mut world::World, &Outbox) + Sync)) {
		self.systems().iter_mut().for_each(
			|r| apply(&mut (**r), world, outbox)
		)
	}

	fn for_each_par_write(&mut self, world: &world::World, apply: &(Fn(&mut systems::System, &world::World) + Sync)) {
		self.systems().par_iter_mut().for_each(
			|r| apply(&mut (**r), world)
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
	// interactions: Vec<Event>,
	//
	camera: math::Inertial<f32>,
	lights: Cycle<Rgba>,
	backgrounds: Cycle<Rgba>,
	speed_factors: Cycle<SpeedFactor>,
	//
	world: world::World,
	bus: PubSub,
	inbox: Inbox,
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
		let mut bus = PubSub::new();
		let inbox = bus.subscribe(Box::new(|e| match e {
			&Message::Alert(_) => true,
			_ => false
		}));

		App {
			viewport: Viewport::rect(w, h, scale),
			input_state: input::InputState::default(),

			camera: Self::init_camera(),
			lights: Self::init_lights(),
			backgrounds: Self::init_backgrounds(),
			speed_factors: Self::init_speed_factors(),

			world: world::World::new(resource_loader, minion_gene_pool),
			bus,
			inbox,
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
		// TODO: send a message to the system instead (how does it reply though?)
		self.systems.physics.read().unwrap().pick(pos)
	}

	fn randomize_minion(&mut self, pos: Position) {
		self.world.randomize_minion(pos, None);
	}

	fn new_minion(&mut self, pos: Position) {
		self.world.new_minion(pos, None);
	}

	fn primary_fire(&mut self, bullet_speed: f32, rate: SecondsValue) {
		// TODO: send a message to the system instead
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
		self.bus.post(e.into());
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

impl AlertReceiver for App {
	fn alert(&mut self, alert: Alert) {
		self.bus.post(alert.into());
	}
}

impl App {
	pub fn init(&mut self) {
		self.init_systems();
		self.alert(world::alert::Alert::BeginSimulation);
	}

	fn register_all(&mut self) {
		// registered() drains the list, so this can be called only once per frame
		let found: Vec<agent::Agent> = self.world.registered()
			.into_iter()
			.filter_map(|id| self.world.agent(*id))
			.map(|a| a.clone())
			.collect();
		self.systems.register(&found[..]);
	}

	fn cleanup_before(&mut self) {
		self.world.cleanup_before();
		self.systems.unregister(&self.world.sweep());
	}

	fn init_systems(&mut self) {
		self.systems.attach(&mut self.bus);
		self.systems.init(&self.world);
	}

	fn update_systems(&mut self, dt: Seconds) {
		self.systems.for_each_par_write(&self.world, &|s, world| s.step(&world, dt));
		self.systems.for_each_read(&mut self.world, &self.bus, &|s, mut world, outbox| s.apply(&mut world, outbox));
	}

	fn cleanup_after(&mut self) {
		self.register_all();
		// self.world.cleanup_after();
	}

	fn tick(&mut self, dt: Seconds) {
		self.world.tick(dt);
	}

	pub fn play_alerts<P, E>(&mut self, alert_player: &mut P)
		where P: ui::AlertPlayer<world::alert::Alert, E> + ui::AlertPlayer<Event, E>,
			  E: Debug {
		for alert in self.inbox.drain().into_iter() {
			match alert {
				Message::Event(ref alert) =>
					match alert_player.play(alert) {
						Err(e) => error!("Unable to play alert {:?}", e),
						Ok(_) => ()
					}
				Message::Alert(ref alert) =>
					match alert_player.play(alert) {
						Err(e) => error!("Unable to play interaction {:?}", e),
						Ok(_) => ()
					}
				_ => {}
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
	fn to_world(&self, view_position: Position) -> Position {
		view_position + self.position()
	}
}

