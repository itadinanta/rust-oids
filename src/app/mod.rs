use std::time::{SystemTime, Duration};
use rand;
use rand::Rng;
use glutin;
use cgmath;
use cgmath::{Matrix4, EuclideanVector};
use render;
use wrapped2d::user_data::NoUserData;

pub struct InputState {
	left_button_pressed: bool,
	mouse_position: b2::Vec2,
}

use wrapped2d::b2;
use std::f64::consts;

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

trait System {
}

pub struct PhysicsSystem {
	world: b2::World<NoUserData>,
}

impl System for PhysicsSystem {}

impl PhysicsSystem {
	pub fn new() -> Self {
		PhysicsSystem { world: new_world() }
	}

	fn new_ball(&mut self, pos: b2::Vec2) {
		let mut world = &self.world;
		let mut rng = rand::thread_rng();
		let radius: f32 = (rng.gen::<f32>() * 1.0) + 1.0;

		let mut circle_shape = b2::CircleShape::new();
		circle_shape.set_radius(radius);

		let mut f_def = b2::FixtureDef::new();
		f_def.density = (rng.gen::<f32>() * 1.0) + 1.0;
		f_def.restitution = 0.2;
		f_def.friction = 0.3;

		let mut b_def = b2::BodyDef::new();
		b_def.body_type = b2::BodyType::Dynamic;
		b_def.position = pos;
		let handle = world.create_body(&b_def);
		world.body_mut(handle)
		     .create_fixture(&circle_shape, &mut f_def);
	}
}

pub struct Position {
	pub x: f32,
	pub y: f32,
}

pub struct Size {
	pub width: f32,
	pub height: f32,
}

pub struct Transform {
	pub position: Position,
	pub angle: f32,
	pub scale: f32,
}

pub type Rgba = [f32; 4];
pub type Id = usize;
pub type PhysicsHandle = Id;

pub enum Shape {
	Ball {
		radius: f32,
	},
	Box {
		size: Size,
	},
}

pub struct GameObject {
	pub id: Id,
	pub transform: Transform,
	pub shape: Shape,
	pub physics_handle: PhysicsHandle,
}

struct GameObjectState {
	transform: Matrix4<f32>,
	physics_handle: Option<PhysicsHandle>,
}

trait Drawable {
	fn transform(&self) -> Transform;
	fn shape(&self) -> Shape;
	fn color(&self) -> Rgba;
}

pub struct App {
	pub viewport: Viewport,
	input_state: InputState,
	wall_clock_start: SystemTime,
	frame_count: u32,
	frame_start: SystemTime,
	frame_elapsed: f32,
	frame_smooth: Smooth<f32>,
	is_running: bool,
	lights: Cycle<[f32; 4]>,
	backgrounds: Cycle<[f32; 4]>,
	//
	physics_system: PhysicsSystem,
}

pub struct Environment {
	pub light: [f32; 4],
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
			input_state: InputState {
				left_button_pressed: false,
				mouse_position: b2::Vec2 { x: 0.0, y: 0.0 },
			},
			lights: Self::init_lights(),
			backgrounds: Self::init_backgrounds(),
			physics_system: PhysicsSystem::new(),
			frame_count: 0u32,
			frame_elapsed: 0.0f32,
			frame_start: SystemTime::now(),
			wall_clock_start: SystemTime::now(),
			frame_smooth: Smooth::new(120),
			is_running: true,
		}
	}

	fn init_lights() -> Cycle<[f32; 4]> {
		Cycle::new(&[[0.001, 0.001, 0.001, 1.0],
		             [0.01, 0.01, 0.01, 1.0],
		             [0.1, 0.1, 0.1, 1.0],
		             [0.31, 0.31, 0.31, 0.5],
		             [1.0, 1.0, 1.0, 1.0],
		             [3.1, 3.1, 3.1, 1.0],
		             [10.0, 10.0, 10.0, 1.0],
		             [31.0, 31.0, 31.0, 1.0],
		             [100.0, 100.0, 100.0, 1.0]])
	}

	fn init_backgrounds() -> Cycle<[f32; 4]> {
		Cycle::new(&[[0., 0., 0., 1.0],
		             [0.01, 0.01, 0.01, 1.0],
		             [0.1, 0.1, 0.1, 1.0],
		             [0.5, 0.5, 0.5, 0.5],
		             [1.0, 1.0, 1.0, 1.0],
		             [3.1, 3.1, 3.1, 1.0],
		             [10.0, 10.0, 10.0, 1.0]])
	}


	fn on_click(&mut self, btn: glutin::MouseButton, pos: b2::Vec2) {
		match btn {
			glutin::MouseButton::Left => {
				self.input_state.left_button_pressed = true;
				self.new_ball(pos);
			}
			_ => (),
		}
	}

	fn new_ball(&self, pos: b2::Vec2) {
		self.physics_system.new_ball(pos);
	}

	fn on_drag(&mut self, pos: b2::Vec2) {
		self.new_ball(pos);
	}

	fn on_release(&mut self, btn: glutin::MouseButton, _: b2::Vec2) {
		match btn {
			glutin::MouseButton::Left => {
				self.input_state.left_button_pressed = false;
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
				let pos = self.input_state.mouse_position;
				self.on_release(b, pos);
			}
			glutin::Event::MouseInput(glutin::ElementState::Pressed, b) => {
				let pos = self.input_state.mouse_position;
				self.on_click(b, pos);
			}
			glutin::Event::MouseMoved(x, y) => {
				fn transform_pos(viewport: &Viewport, x: u32, y: u32) -> b2::Vec2 {
					let (tx, ty) = viewport.to_world(x, y);
					return b2::Vec2 { x: tx, y: ty };
				}
				let pos = transform_pos(&self.viewport, x as u32, y as u32);
				self.input_state.mouse_position = pos;
				if self.input_state.left_button_pressed {
					self.on_drag(pos);
				}
			}
			_ => (),
		}
	}

	pub fn on_resize(&mut self, width: u32, height: u32) {
		self.viewport = Viewport::rect(width, height, self.viewport.scale);
	}

	pub fn render(&self, renderer: &mut render::Draw) {
		for (_, b) in self.world.bodies() {
			let body = b.borrow();
			let position = (*body).position();
			let angle = (*body).angle() as f32;
			use cgmath::Rotation3;
			let body_rot = Matrix4::from(cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(),
			                                                                 cgmath::rad(angle)));
			let body_trans = Matrix4::from_translation(cgmath::Vector3::new(position.x, position.y, 0.0));

			let body_transform = body_trans * body_rot;

			for (_, f) in body.fixtures() {
				let fixture = f.borrow();
				let shape = (*fixture).shape();
				let density = (*fixture).density();

				match *shape {
					b2::UnknownShape::Circle(ref s) => {
						let p = s.position();
						let r = s.radius() as f32;

						let fixture_scale = Matrix4::from_scale(r);
						let fixture_trans = Matrix4::from_translation(cgmath::Vector3::new(p.x, p.y, 0.0));
						let transform = body_transform * fixture_trans * fixture_scale;

						let lightness = 1. - density * 0.5;

						let color = [0., 10. * lightness, 0., 1.];

						renderer.draw_quad(&transform.into(), color);
					}
					b2::UnknownShape::Polygon(_) => {
						// TODO: need to draw fill poly
					}
					_ => (),
				}
			}
		}
	}

	pub fn environment(&self) -> Environment {
		Environment {
			light: self.lights.get(),
			background: self.backgrounds.get(),
		}
	}

	pub fn update(&mut self) -> Result<Update, ()> {
		match self.frame_start.elapsed() {
			Ok(dt) => {
				let frame_time = (dt.as_secs() as f32) + (dt.subsec_nanos() as f32) * 1e-9;
				let frame_time_smooth = self.frame_smooth.smooth(frame_time);
				self.update_physics(frame_time_smooth);
				self.frame_elapsed += frame_time;
				self.frame_start = SystemTime::now();
				self.frame_count += 1;

				Ok(Update {
					wall_clock_elapsed: self.wall_clock_start.elapsed().unwrap_or_else(|_| Duration::new(0, 0)),
					frame_count: self.frame_count,
					frame_elapsed: self.frame_elapsed,
					frame_time: frame_time,
					frame_time_smooth: frame_time_smooth,
					fps: 1.0 / frame_time_smooth,
				})
			}

			Err(_) => Err(()),
		}
	}

	fn update_physics(&mut self, dt: f32) {
		let world = &mut self.world;
		world.step(dt, 8, 3);
		const MAX_RADIUS: f32 = 5.0;
		let (_, edge) = self.viewport.to_world(0, self.viewport.height);
		let mut v = Vec::new();
		for (h, b) in world.bodies() {
			let body = b.borrow();
			let position = (*body).position();
			if position.y < (edge - MAX_RADIUS) {
				v.push(h);
			}
		}
		for h in v {
			world.destroy_body(h);
		}
	}
}

fn new_world() -> b2::World<NoUserData> {
	let mut world = b2::World::new(&b2::Vec2 { x: 0.0, y: -9.8 });

	let mut b_def = b2::BodyDef::new();
	b_def.body_type = b2::BodyType::Static;
	b_def.position = b2::Vec2 { x: 0.0, y: -8.0 };

	let mut ground_box = b2::PolygonShape::new();
	{
		ground_box.set_as_box(20.0, 1.0);
		let ground_handle = world.create_body(&b_def);
		let ground = &mut world.body_mut(ground_handle);
		ground.create_fast_fixture(&ground_box, 0.);

		ground_box.set_as_oriented_box(1.0,
		                               5.0,
		                               &b2::Vec2 { x: 21.0, y: 5.0 },
		                               (-consts::FRAC_PI_8) as f32);
		ground.create_fast_fixture(&ground_box, 0.);

		ground_box.set_as_oriented_box(1.0,
		                               5.0,
		                               &b2::Vec2 { x: -21.0, y: 5.0 },
		                               (consts::FRAC_PI_8) as f32);
		ground.create_fast_fixture(&ground_box, 0.);
	}
	world
}

pub struct Smooth<S: ::num::Num> {
	ptr: usize,
	count: usize,
	acc: S,
	last: S,
	values: Vec<S>,
}

impl<S: ::num::Num + ::num::NumCast + ::std::marker::Copy> Smooth<S> {
	pub fn new(window_size: usize) -> Smooth<S> {
		Smooth {
			ptr: 0,
			count: 0,
			last: S::zero(),
			acc: S::zero(),
			values: vec![S::zero(); window_size],
		}
	}

	pub fn smooth(&mut self, value: S) -> S {
		let len = self.values.len();
		if self.count < len {
			self.count = self.count + 1;
		} else {
			self.acc = self.acc - self.values[self.ptr];
		}
		self.acc = self.acc + value;
		self.values[self.ptr] = value;
		self.ptr = ((self.ptr + 1) % len) as usize;
		self.last = self.acc / ::num::cast(self.count).unwrap();
		self.last
	}
}
