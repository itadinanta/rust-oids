mod mainloop;

use core::math;

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
use backend::obj::{Geometry, Drawable, Transformable};

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
	light_position: obj::Position,
	lights: Cycle<[f32; 4]>,
	backgrounds: Cycle<[f32; 4]>,
	//
	world: world::World,
	physics_system: systems::PhysicsSystem,
	animation_system: systems::AnimationSystem,
}

pub struct Environment {
	pub light: [f32; 4],
	pub light_position: obj::Position,
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
			light_position: obj::Position::new(10.0, 10.0),
			lights: Self::init_lights(),
			backgrounds: Self::init_backgrounds(),

			world: world::World::new(),
			// subsystem, need to update each
			physics_system: systems::PhysicsSystem::new(),
			animation_system: systems::AnimationSystem::new(),

			// runtime and timing
			frame_count: 0u32,
			frame_elapsed: 0.0f32,
			frame_start: SystemTime::now(),
			wall_clock_start: SystemTime::now(),
			frame_smooth: math::Smooth::new(120),
			is_running: true,
		}
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


	fn on_click(&mut self, btn: glutin::MouseButton, pos: obj::Position) {
		match btn {
			glutin::MouseButton::Left => {
				self.input_state.button_press(Left);
				self.new_minion(pos);
			}
			glutin::MouseButton::Right => {
				self.input_state.button_press(Right);
				self.light_position = pos;
				// self.new_star(pos);
			}
			_ => (),
		}
	}

	fn new_resource(&mut self, pos: obj::Position) {
		let id = self.world.new_resource(pos);
		self.register(id);
	}

	fn new_minion(&mut self, pos: obj::Position) {
		let id = self.world.new_minion(pos);
		self.register(id);
	}

	fn register(&mut self, id: obj::Id) {
		let found = self.world.friend_mut(id);
		self.physics_system.register(found.unwrap());
	}

	fn on_left_drag(&mut self, pos: obj::Position) {
		self.new_resource(pos);
	}

	fn on_mouse_move(&mut self, pos: obj::Position) {
		// self.light_position = pos;
	}

	fn on_right_drag(&mut self, pos: obj::Position) {
		self.light_position = pos;
	}

	fn on_release(&mut self, btn: glutin::MouseButton, _: obj::Position) {
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

	pub fn on_keyboard_input(&mut self, e: glutin::Event) {
		match e {
			glutin::Event::ReceivedCharacter(char) => {
				match char {
					_ => println!("Key pressed {:?}", char),
				}
			}
			glutin::Event::KeyboardInput(glutin::ElementState::Pressed, scancode, vk) => {
				match vk {
					Some(glutin::VirtualKeyCode::L) => {
						self.lights.next();
					}
					Some(glutin::VirtualKeyCode::B) => {
						self.backgrounds.next();
					}
					Some(glutin::VirtualKeyCode::Escape) => {
						self.quit();
					}
					_ => println!("Key pressed {:?}/{:?}", scancode, vk),
				}
			}
			_ => (),
		}
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
				fn transform_pos(viewport: &Viewport, x: u32, y: u32) -> obj::Position {
					let (tx, ty) = viewport.to_world(x, y);
					return obj::Position { x: tx, y: ty };
				}
				let pos = transform_pos(&self.viewport, x as u32, y as u32);
				self.input_state.mouse_position_at(pos);
				if self.input_state.button_pressed(Left) {
					self.on_left_drag(pos);
				} else if self.input_state.button_pressed(Right) {
					self.on_right_drag(pos);
				} else {
					self.on_mouse_move(pos);
				}
			}
			_ => (),
		}
	}

	pub fn on_resize(&mut self, width: u32, height: u32) {
		self.viewport = Viewport::rect(width, height, self.viewport.scale);
	}

	pub fn render(&self, renderer: &mut render::Draw) {
		for (_, b) in self.world.minions.agents() {
			for segment in b.segments() {
				let transform = segment.transform();
				let position = transform.position;
				let angle = transform.angle;

				use cgmath::Rotation3;
				let body_rot = Matrix4::from(cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(),
				                                                                 cgmath::rad(angle)));
				let body_trans = Matrix4::from_translation(cgmath::Vector3::new(position.x, position.y, 0.0));

				let body_transform = body_trans * body_rot;

				let mesh = &segment.mesh();
				let fixture_scale = Matrix4::from_scale(mesh.shape.radius());
				let transform = body_transform * fixture_scale;

				match mesh.shape {
					obj::Shape::Ball { .. } => {
						renderer.draw_ball(&transform.into(), segment.color());
					}
					obj::Shape::Star { .. } => {
						renderer.draw_star(&transform.into(), &mesh.vertices[..], segment.color());
					}
					obj::Shape::Box { ratio, .. } => {
						renderer.draw_quad(&transform.into(), ratio, segment.color());
					}
					obj::Shape::Triangle { .. } => {
						renderer.draw_star(&transform.into(), &mesh.vertices[0..3], segment.color());
					}
				}
			}
		}
	}

	pub fn environment(&self) -> Environment {
		Environment {
			light: self.lights.get(),
			light_position: self.light_position,
			background: self.backgrounds.get(),
		}
	}

	fn update_systems(&mut self, dt: f32) {
		self.update_physics(dt);
		self.update_animation(dt);
	}

	pub fn update(&mut self) -> Result<Update, SystemTimeError> {
		self.frame_start.elapsed().map(|dt| {
			let frame_time = (dt.as_secs() as f32) + (dt.subsec_nanos() as f32) * 1e-9;
			let frame_time_smooth = self.frame_smooth.smooth(frame_time);


			self.frame_elapsed += frame_time;
			self.frame_start = SystemTime::now();

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

	fn update_physics(&mut self, dt: f32) {
		let (_, edge) = self.viewport.to_world(0, self.viewport.height);
		self.physics_system.drop_below(edge);
		self.physics_system.follow_me(self.light_position);
		self.physics_system.update_world(dt, &mut self.world);
	}

	fn update_animation(&mut self, dt: f32) {
		self.animation_system.update_world(dt, &mut self.world);
	}
}
