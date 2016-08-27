mod main;
mod ev;

use std::time;

use core::util::Cycle;
use core::geometry::*;
use core::clock::*;
use core::math;
use core::math::Directional;
use core::math::Smooth;

use backend::obj;
use backend::obj::*;
use backend::world;
use backend::systems;
use backend::systems::System;

use frontend::input;
use frontend::render;

use cgmath;
use cgmath::{Matrix4, SquareMatrix};

pub enum Event {
	CamUp,
	CamDown,
	CamLeft,
	CamRight,

	CamReset,

	NextLight,
	PrevLight,

	NextBackground,
	PrevBackground,

	Reload,

	AppQuit,

	MoveLight(Position),
	NewMinion(Position),
	NewResource(Position),
}

pub fn run() {
	main::main_loop();
}

pub struct Viewport {
	width: u32,
	height: u32,
	pub ratio: f32,
	pub scale: f32,
}

impl Viewport {
	fn rect(w: u32, h: u32, scale: f32) -> Viewport {
		Viewport {
			width: w,
			height: h,
			ratio: (w as f32 / h as f32),
			scale: scale,
		}
	}

	fn to_world(&self, pos: &Position) -> Position {
		let dx = self.width as f32 / self.scale;
		let tx = (pos.x - (self.width as f32 * 0.5)) / dx;
		let ty = ((self.height as f32 * 0.5) - pos.y) / dx;
		Position::new(tx, ty)
	}
}

pub struct Systems {
	physics: systems::PhysicsSystem,
	animation: systems::AnimationSystem,
	game: systems::GameSystem,
	ai: systems::AiSystem,
}

impl Systems {
	fn systems(&mut self) -> Vec<&mut systems::System> {
		vec![&mut self.animation as &mut systems::System,
		     &mut self.game as &mut systems::System,
		     &mut self.ai as &mut systems::System,
		     &mut self.physics as &mut systems::System]
	}

	fn for_each(&mut self, apply: &Fn(&mut systems::System)) {
		for r in self.systems().as_mut_slice() {
			apply(*r);
		}
	}

	fn from_world(&mut self, world: &world::World, apply: &Fn(&mut systems::System, &world::World)) {
		for r in self.systems().as_mut_slice() {
			apply(*r, &world);
		}
	}

	fn to_world(&mut self, mut world: &mut world::World, apply: &Fn(&mut systems::System, &mut world::World)) {
		for r in self.systems().as_mut_slice() {
			apply(*r, &mut world);
		}
	}
}

pub struct App {
	pub viewport: Viewport,
	input_state: input::InputState,
	wall_clock_start: SystemStopwatch,
	frame_count: u32,
	frame_start: SystemStopwatch,
	frame_elapsed: f32,
	frame_smooth: math::MovingAverage<f32>,
	is_running: bool,
	//
	light_position: Position,
	camera: math::Inertial<f32>,
	lights: Cycle<[f32; 4]>,
	backgrounds: Cycle<[f32; 4]>,
	//
	world: world::World,
	systems: Systems,
}

pub struct Environment {
	pub light: [f32; 4],
	pub light_position: Position,
	pub background: [f32; 4],
}

pub struct Update {
	pub frame_count: u32,
	pub wall_clock_elapsed: f32,
	pub frame_elapsed: f32,
	pub frame_time: f32,
	pub frame_time_smooth: f32,
	pub fps: f32,
}

impl App {
	pub fn new(w: u32, h: u32, scale: f32) -> App {
		App {
			viewport: Viewport::rect(w, h, scale),
			input_state: input::InputState::default(),

			// testbed, will need a display/render subsystem
			light_position: Position::new(10.0, 10.0),
			camera: Self::init_camera(),
			lights: Self::init_lights(),
			backgrounds: Self::init_backgrounds(),

			world: world::World::new(),
			// subsystem, need to update each
			systems: Systems {
				physics: systems::PhysicsSystem::new(),
				animation: systems::AnimationSystem::new(),
				game: systems::GameSystem::new(),
				ai: systems::AiSystem::new(),
			},
			// runtime and timing
			frame_count: 0u32,
			frame_elapsed: 0.0f32,
			frame_start: SystemStopwatch::new(),
			wall_clock_start: time::SystemTime::now(),
			frame_smooth: math::MovingAverage::new(120),
			is_running: true,
		}
	}


	fn init_camera() -> math::Inertial<f32> {
		math::Inertial::new(10.0, 1. / 180., 0.5)
	}

	fn init_lights() -> Cycle<[f32; 4]> {
		Cycle::new(&[[1.0, 1.0, 1.0, 1.0],
		             [3.1, 3.1, 3.1, 1.0],
		             [10.0, 10.0, 10.0, 1.0],
		             [31.0, 31.0, 31.0, 1.0],
		             [100.0, 100.0, 100.0, 1.0],
		             [0.001, 0.001, 0.001, 1.0],
		             [0.01, 0.01, 0.01, 1.0],
		             [0.1, 0.1, 0.1, 1.0],
		             [0.31, 0.31, 0.31, 0.5]])
	}

	fn init_backgrounds() -> Cycle<[f32; 4]> {
		Cycle::new(&[[0.05, 0.07, 0.1, 1.0],
		             [0.5, 0.5, 0.5, 0.5],
		             [1.0, 1.0, 1.0, 1.0],
		             [3.1, 3.1, 3.1, 1.0],
		             [10.0, 10.0, 10.0, 1.0],
		             [0., 0., 0., 1.0],
		             [0.01, 0.01, 0.01, 1.0]])
	}

	fn new_resource(&mut self, pos: Position) {
		let id = self.world.new_resource(pos);
		self.register(id);
	}

	fn new_minion(&mut self, pos: Position) {
		let id = self.world.new_minion(pos);
		self.register(id);
	}

	fn register(&mut self, id: obj::Id) {
		let found = self.world.friend_mut(id);
		self.systems.physics.register(found.unwrap());
	}

	pub fn on_app_event(&mut self, e: Event) {
		match e {
			Event::CamUp => self.camera.push(math::Direction::Up),
			Event::CamDown => self.camera.push(math::Direction::Down),
			Event::CamLeft => self.camera.push(math::Direction::Left),
			Event::CamRight => self.camera.push(math::Direction::Right),

			Event::CamReset => {
				self.camera.reset();
			}
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

			Event::Reload => {}

			Event::AppQuit => self.quit(),

			Event::MoveLight(pos) => self.light_position = pos,
			Event::NewMinion(pos) => self.new_minion(pos),
			Event::NewResource(pos) => self.new_resource(pos),
		}
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

	fn update_input(&mut self, _: f32) {
		let mut events = Vec::new();

		macro_rules! on_key_held {
			[$($key:ident -> $app_event:ident),*] => (
				$(if self.input_state.key_pressed(input::Key::$key) { events.push(Event::$app_event); })
				*
			)
		}
		macro_rules! on_key_pressed_once {
			[$($key:ident -> $app_event:ident),*] => (
				$(if self.input_state.key_once(input::Key::$key) { events.push(Event::$app_event); })
				*
			)
		}
		on_key_held! [
			Up -> CamUp,
			Down -> CamDown,
			Left -> CamLeft,
			Right-> CamRight
		];

		on_key_pressed_once! [
			F5 -> Reload,
			N0 -> CamReset,
			Home -> CamReset,
			KpHome -> CamReset,
			L -> NextLight,
			B -> NextBackground,
			K -> PrevLight,
			V -> PrevBackground,
			Esc -> AppQuit
		];

		let mouse_pos = self.input_state.mouse_position();
		let view_pos = self.to_view(&mouse_pos);
		let world_pos = self.to_world(&view_pos);

		if self.input_state.key_once(input::Key::MouseRight) {
			if self.input_state.any_ctrl_pressed() {
				events.push(Event::NewResource(world_pos));
			} else {
				events.push(Event::NewMinion(world_pos));
			}
		}

		if self.input_state.key_pressed(input::Key::MouseLeft) {
			events.push(Event::MoveLight(world_pos));
		}

		for e in events {
			self.on_app_event(e)
		}
	}

	fn to_view(&self, pos: &Position) -> Position {
		self.viewport.to_world(pos)
	}

	fn to_world(&self, t: &Position) -> Position {
		t + self.camera.position()
	}

	pub fn on_resize(&mut self, width: u32, height: u32) {
		self.viewport = Viewport::rect(width, height, self.viewport.scale);
	}

	fn from_transform(transform: &Transform) -> Matrix4<f32> {
		use cgmath::Rotation3;
		let position = transform.position;
		let angle = transform.angle;
		let rot = Matrix4::from(cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::rad(angle)));
		let trans = Matrix4::from_translation(cgmath::Vector3::new(position.x, position.y, 0.0));

		trans * rot
	}

	fn from_position(position: &Position) -> Matrix4<f32> {
		Matrix4::from_translation(cgmath::Vector3::new(position.x, position.y, 0.0))
	}

	fn render_minions(&self, renderer: &mut render::Draw) {
		for (_, b) in self.world.minions.agents() {
			for segment in b.segments() {
				let body_transform = Self::from_transform(&segment.transform());

				let mesh = &segment.mesh();
				let fixture_scale = Matrix4::from_scale(mesh.shape.radius());
				let transform = body_transform * fixture_scale;

				match mesh.shape {
					obj::Shape::Ball { .. } => {
						renderer.draw_ball(&transform, segment.color());
					}
					obj::Shape::Star { .. } => {
						renderer.draw_star(&transform, &mesh.vertices[..], segment.color());
					}
					obj::Shape::Box { ratio, .. } => {
						renderer.draw_quad(&transform, ratio, segment.color());
					}
					obj::Shape::Triangle { .. } => {
						renderer.draw_triangle(&transform, &mesh.vertices[0..3], segment.color());
					}
				}
			}
		}
	}

	fn render_extent(&self, renderer: &mut render::Draw) {
		let extent = &self.world.extent;
		let points = &[extent.min,
		               Position::new(extent.min.x, extent.max.y),
		               extent.max,
		               Position::new(extent.max.x, extent.min.y),
		               extent.min];
		let transform = Matrix4::identity();
		renderer.draw_lines(&transform, points, self.lights.get());
	}

	fn render_hud(&self, renderer: &mut render::Draw) {
		let transform = Self::from_position(&self.light_position);
		renderer.draw_ball(&transform, self.lights.get());
	}

	pub fn render(&self, renderer: &mut render::Draw) {
		self.render_minions(renderer);
		self.render_extent(renderer);
		self.render_hud(renderer);
	}

	pub fn environment(&self) -> Environment {
		Environment {
			light: self.lights.get(),
			light_position: self.light_position,
			background: self.backgrounds.get(),
		}
	}

	pub fn init(&mut self) {
		self.init_systems();
	}

	fn init_systems(&mut self) {
		self.systems.from_world(&self.world, &|s, world| s.init(&world));
	}

	fn cleanup(&mut self) {
		let freed = self.world.cleanup().freed;
		self.systems.for_each(&|s| for freed_agent in freed.iter() {
			s.unregister(freed_agent);
		});
	}

	fn update_systems(&mut self, dt: f32) {
		self.systems.ai.follow_me(self.light_position);
		self.systems.to_world(&mut self.world,
		                      &|s, mut world| s.update_world(dt, &mut world));
	}

	pub fn update(&mut self) -> Update {
		let frame_time = self.frame_start.seconds();
		let frame_time_smooth = self.frame_smooth.smooth(frame_time);

		self.frame_elapsed += frame_time;
		self.frame_start.reset();

		self.cleanup();

		self.camera.update(frame_time_smooth);

		self.update_input(frame_time_smooth);
		self.update_systems(frame_time_smooth);
		self.frame_count += 1;

		Update {
			wall_clock_elapsed: self.wall_clock_start.seconds(),
			frame_count: self.frame_count,
			frame_elapsed: self.frame_elapsed,
			frame_time: frame_time,
			frame_time_smooth: frame_time_smooth,
			fps: 1.0 / frame_time_smooth,
		}
	}
}
