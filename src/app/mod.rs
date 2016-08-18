mod mainloop;
mod ev;
use core::math;
use core::math::Directional;

use backend::obj;
use backend::world;
use backend::systems;

use backend::systems::System;

use frontend::input;
use frontend::render;

use std::time::{SystemTime, Duration, SystemTimeError};
use glutin;
use cgmath;
use cgmath::Matrix4;
use frontend::input::Button::*;
use backend::obj::*;
use core::geometry::*;

pub fn run() {
	mainloop::main_loop();
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

	fn to_world(&self, x: u32, y: u32) -> (f32, f32) {
		let dx = self.width as f32 / self.scale;
		let tx = (x as f32 - (self.width as f32 * 0.5)) / dx;
		let ty = ((self.height as f32 * 0.5) - y as f32) / dx;
		(tx, ty)
	}
}

struct Cycle<T: Copy> {
	items: Vec<T>,
	index: usize,
}

impl<T> Cycle<T>
    where T: Copy
{
	pub fn new(items: &[T]) -> Cycle<T> {
		Cycle {
			items: items.to_vec(),
			index: 0,
		}
	}

	pub fn get(&self) -> T {
		self.items[self.index]
	}

	pub fn next(&mut self) -> T {
		self.index = (self.index + 1) % self.items.len();
		self.items[self.index]
	}
}

pub struct App {
	pub viewport: Viewport,
	input_state: input::InputState,
	wall_clock_start: SystemTime,
	frame_count: u32,
	frame_start: SystemTime,
	frame_elapsed: f32,
	frame_smooth: math::Smooth<f32>,
	is_running: bool,
	//
	light_position: Position,
	camera: math::Inertial<f32>,
	lights: Cycle<[f32; 4]>,
	backgrounds: Cycle<[f32; 4]>,
	//
	world: world::World,

	physics: systems::PhysicsSystem,
	animation: systems::AnimationSystem,
	game: systems::GameSystem,
	ai: systems::AiSystem,
}

pub struct Environment {
	pub light: [f32; 4],
	pub light_position: Position,
	pub background: [f32; 4],
}

pub struct Update {
	pub frame_count: u32,
	pub wall_clock_elapsed: Duration,
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
			physics: systems::PhysicsSystem::new(),
			animation: systems::AnimationSystem::new(),
			game: systems::GameSystem::new(),
			ai: systems::AiSystem::new(),

			// runtime and timing
			frame_count: 0u32,
			frame_elapsed: 0.0f32,
			frame_start: SystemTime::now(),
			wall_clock_start: SystemTime::now(),
			frame_smooth: math::Smooth::new(120),
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


	fn on_click(&mut self, btn: glutin::MouseButton, pos: Position) {
		match btn {
			glutin::MouseButton::Left => {
				self.input_state.button_press(Left);
				self.light_position = pos;
			}
			glutin::MouseButton::Right => {
				self.input_state.button_press(Right);
				self.new_minion(pos);
			}
			_ => (),
		}
	}

	fn on_mouse_move(&mut self, btn: Option<glutin::MouseButton>, pos: Position) {
		match btn {
			Some(glutin::MouseButton::Left) => {
				self.light_position = pos;
			}
			Some(glutin::MouseButton::Right) => {
				let id = self.world.new_resource(pos);
				self.register(id);
			}
			_ => (),
		}
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
		self.physics.register(found.unwrap());
	}

	fn on_release(&mut self, btn: glutin::MouseButton, _: Position) {
		match btn {
			glutin::MouseButton::Left => {
				self.input_state.button_release(Left);
			}

			glutin::MouseButton::Right => {
				self.input_state.button_release(Right);
			}

			_ => (),
		}
	}

	pub fn on_app_event(&mut self, e: ev::Event) {
		match e {
			ev::Event::CamUp => self.camera.push(math::Direction::Up),
			ev::Event::CamDown => self.camera.push(math::Direction::Down),
			ev::Event::CamLeft => self.camera.push(math::Direction::Left),
			ev::Event::CamRight => self.camera.push(math::Direction::Right),

			ev::Event::CamReset => self.camera.reset(),

			ev::Event::NextLight => {
				self.lights.next();
			}
			ev::Event::PrevLight => {
				self.lights.next();
			}

			ev::Event::NextBackground => {
				self.backgrounds.next();
			}
			ev::Event::PrevBackground => {
				self.backgrounds.next();
			}

			ev::Event::Reload => {}

			ev::Event::AppQuit => self.quit(),

			ev::Event::MoveLight(_, _) => {}
			ev::Event::NewMinion(_, _) => {}

			_ => {}
		}
	}

	fn keymap(e: glutin::Event) -> ev::Event {
		match e {
			glutin::Event::ReceivedCharacter(char) => {
				match char {
					_ => {
						println!("Key pressed {:?}", char);
						ev::Event::NoEvent
					}
				}
			}
			glutin::Event::KeyboardInput(glutin::ElementState::Pressed, scancode, vk) => {
				match vk {
					Some(glutin::VirtualKeyCode::Up) => ev::Event::CamUp,
					Some(glutin::VirtualKeyCode::Down) => ev::Event::CamDown,
					Some(glutin::VirtualKeyCode::Right) => ev::Event::CamRight,
					Some(glutin::VirtualKeyCode::Left) => ev::Event::CamLeft,
					Some(glutin::VirtualKeyCode::Home) => ev::Event::CamReset,

					Some(glutin::VirtualKeyCode::L) => ev::Event::NextLight,
					Some(glutin::VirtualKeyCode::B) => ev::Event::NextBackground,
					Some(glutin::VirtualKeyCode::Escape) => ev::Event::AppQuit,
					_ => {
						println!("No mapping for {:?}/{:?}", scancode, vk);
						ev::Event::NoEvent
					}
				}
			}
			_ => ev::Event::NoEvent,
		}
	}

	pub fn on_keyboard_input(&mut self, e: glutin::Event) {
		self.on_app_event(Self::keymap(e))
	}

	pub fn quit(&mut self) {
		self.is_running = false;
	}

	pub fn is_running(&self) -> bool {
		self.is_running
	}

	pub fn on_mouse_input(&mut self, e: glutin::Event) {
		match e {
			glutin::Event::MouseInput(glutin::ElementState::Released, b) => {
				let pos = self.input_state.mouse_position();
				self.on_release(b, pos);
			}
			glutin::Event::MouseInput(glutin::ElementState::Pressed, b) => {
				let pos = self.input_state.mouse_position();
				self.on_click(b, pos);
			}
			glutin::Event::MouseMoved(x, y) => {
				let pos = self.to_world(x as u32, y as u32);

				self.input_state.mouse_position_at(pos);
				if self.input_state.button_pressed(Left) {
					self.on_mouse_move(Some(glutin::MouseButton::Left), pos);
				} else if self.input_state.button_pressed(Right) {
					self.on_mouse_move(Some(glutin::MouseButton::Right), pos);
				} else {
					self.on_mouse_move(None, pos);
				}
			}
			_ => (),
		}
	}

	fn to_world(&self, x: u32, y: u32) -> Position {
		let (tx, ty) = self.viewport.to_world(x, y);
		let cgmath::Point2 { x: cx, y: cy } = self.camera.position();
		Position {
			x: tx + cx,
			y: ty + cy,
		}
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

	fn render_extent(&self, renderer: &mut render::Draw) {}

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

	fn update_systems(&mut self, dt: f32) {
		self.animation.update_world(dt, &mut self.world);

		self.game.update_world(dt, &mut self.world);

		self.ai.follow_me(self.light_position);
		self.ai.update_world(dt, &mut self.world);

		self.physics.update_world(dt, &mut self.world);
	}

	fn init_systems(&mut self) {
		self.animation.init(&mut self.world);

		self.ai.init(&mut self.world);

		self.game.init(&mut self.world);

		self.physics.init(&mut self.world);
	}

	pub fn update(&mut self) -> Result<Update, SystemTimeError> {
		self.frame_start.elapsed().map(|dt| {
			let frame_time = (dt.as_secs() as f32) + (dt.subsec_nanos() as f32) * 1e-9;
			let frame_time_smooth = self.frame_smooth.smooth(frame_time);


			self.frame_elapsed += frame_time;
			self.frame_start = SystemTime::now();

			self.camera.update(frame_time_smooth);
			self.update_systems(frame_time_smooth);
			self.frame_count += 1;

			Update {
				wall_clock_elapsed: self.wall_clock_start.elapsed().unwrap_or_else(|_| Duration::new(0, 0)),
				frame_count: self.frame_count,
				frame_elapsed: self.frame_elapsed,
				frame_time: frame_time,
				frame_time_smooth: frame_time_smooth,
				fps: 1.0 / frame_time_smooth,
			}
		})
	}
}
