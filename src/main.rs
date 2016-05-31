mod render;

extern crate rand;
extern crate num;
extern crate gfx_text;

#[macro_use]
extern crate log;
extern crate simplelog;

extern crate cgmath;

extern crate graphics;
extern crate wrapped2d;

#[macro_use]
extern crate gfx;
extern crate gfx_device_gl;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate genmesh;
extern crate piston;

use std::time::SystemTime;
use rand::Rng;

use piston::event_loop::*;
use piston::input::*;

use gfx::Device;
use gfx::traits::FactoryExt;

use cgmath::{Matrix4, EuclideanVector};

use wrapped2d::b2;
use std::f64::consts;

pub struct Viewport {
	width: u32,
	height: u32,
	ratio: f32,
	scale: f32,
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

pub struct InputState {
	left_button_pressed: bool,
	mouse_position: b2::Vec2,
}

use render::VertexPosNormal as Vertex;
use render::ColorFormat;
use render::DepthFormat;

const QUAD: [Vertex; 6] = [Vertex {
	                           pos: [-1.0, -1.0, 0.0],
	                           normal: [0.0, 0.0, 1.0],
	                           tangent: [1.0, 0.0, 0.0],
	                           tex_coord: [0.0, 0.0],
                           },
                           Vertex {
	                           pos: [1.0, -1.0, 0.0],
	                           normal: [0.0, 0.0, 1.0],
	                           tangent: [1.0, 0.0, 0.0],
	                           tex_coord: [1.0, 0.0],
                           },
                           Vertex {
	                           pos: [1.0, 1.0, 0.0],
	                           normal: [0.0, 0.0, 1.0],
	                           tangent: [1.0, 0.0, 0.0],
	                           tex_coord: [1.0, 1.0],
                           },
                           Vertex {
	                           pos: [-1.0, -1.0, 0.0],
	                           normal: [0.0, 0.0, 1.0],
	                           tangent: [1.0, 0.0, 0.0],
	                           tex_coord: [0.0, 0.0],
                           },
                           Vertex {
	                           pos: [-1.0, 1.0, 0.0],
	                           normal: [0.0, 0.0, 1.0],
	                           tangent: [1.0, 0.0, 0.0],
	                           tex_coord: [0.0, 1.0],
                           },
                           Vertex {
	                           pos: [1.0, 1.0, 0.0],
	                           normal: [0.0, 0.0, 1.0],
	                           tangent: [1.0, 0.0, 0.0],
	                           tex_coord: [1.0, 1.0],
                           }];

fn new_ball(world: &mut b2::World, pos: b2::Vec2) {
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
	world.get_body_mut(handle)
		.create_fixture(&circle_shape, &mut f_def);
}
struct App {
	viewport: Viewport,
	input_state: InputState,
	world: b2::World,
}

impl App {
	fn on_click(&mut self, btn: glutin::MouseButton, pos: b2::Vec2) {
		match btn {
			glutin::MouseButton::Left => {
				self.input_state.left_button_pressed = true;
				new_ball(&mut self.world, pos);
			}
			_ => (),
		}
	}

	fn on_drag(&mut self, pos: b2::Vec2) {
		new_ball(&mut self.world, pos);
	}

	fn on_release(&mut self, btn: glutin::MouseButton, _: b2::Vec2) {
		match btn {
			glutin::MouseButton::Left => {
				self.input_state.left_button_pressed = false;
			}
			_ => (),
		}
	}

	fn mouse_input(&mut self, e: glutin::Event) {
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

	fn on_resize(&mut self, width: u32, height: u32) {
		self.viewport = Viewport::rect(width, height, self.viewport.scale);
	}

	fn render<R: gfx::Resources, C: gfx::CommandBuffer<R>>(&self,
	                                                       renderer: &render::DrawShaded<R>,
	                                                       mut encoder: &mut gfx::Encoder<R, C>,
	                                                       vertices: &gfx::handle::Buffer<R, Vertex>,
	                                                       indices: &gfx::Slice<R>,
	                                                       color: &gfx::handle::RenderTargetView<R, ColorFormat>,
	                                                       depth: &gfx::handle::DepthStencilView<R, DepthFormat>) {
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

						renderer.draw(&mut encoder,
						              &vertices,
						              &indices,
						              &transform.into(),
						              &color,
						              &depth);
					}
					b2::UnknownShape::Polygon(_) => {
						// TODO: need to draw fill poly
					}
					_ => (),
				}
			}
		}

	}

	fn update(&mut self, dt: f32) {
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

fn new_world() -> b2::World {
	let mut world = b2::World::new(&b2::Vec2 { x: 0.0, y: -9.8 });

	let mut b_def = b2::BodyDef::new();
	b_def.body_type = b2::BodyType::Static;
	b_def.position = b2::Vec2 { x: 0.0, y: -8.0 };

	let mut ground_box = b2::PolygonShape::new();
	{
		ground_box.set_as_box(20.0, 1.0);
		let ground_handle = world.create_body(&b_def);
		let ground = &mut world.get_body_mut(ground_handle);
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

pub struct Smooth<S: num::Num> {
	ptr: usize,
	count: usize,
	acc: S,
	values: Vec<S>,
}

impl<S: num::Num + num::NumCast + std::marker::Copy> Smooth<S> {
	pub fn new(window_size: usize) -> Smooth<S> {
		Smooth {
			ptr: 0,
			count: 0,
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
		self.acc / num::cast(self.count).unwrap()
	}
}

fn main() {
	const WIDTH: u32 = 1280;
	const HEIGHT: u32 = 720;

	let builder = glutin::WindowBuilder::new()
		.with_title("Box2d + GFX".to_string())
		.with_dimensions(WIDTH, HEIGHT)
		.with_vsync();

	let (window, mut device, mut factory, mut main_color, mut main_depth) =
		gfx_window_glutin::init::<ColorFormat, DepthFormat>(builder);
	let (w, h, _, _) = main_color.get_dimensions();

	let renderer = render::DrawShaded::new(&mut factory);

	// Create a new game and run it.
	let mut app = App {
		viewport: Viewport::rect(w as u32, h as u32, 50.0),
		input_state: InputState {
			left_button_pressed: false,
			mouse_position: b2::Vec2 { x: 0.0, y: 0.0 },
		},
		world: new_world(),
	};

	let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&QUAD, ());

	let mut text_renderer = gfx_text::new(factory.clone()).build().unwrap();

	let lights: Vec<render::PointLight> = vec![render::PointLight {
		                                           propagation: [0.3, 0.5, 0.4, 0.0],
		                                           center: [-15.0, -5.0, 1.0, 1.0],
		                                           color: [1.0, 0.0, 0.0, 1.0],
	                                           },
	                                           render::PointLight {
		                                           propagation: [0.5, 0.5, 0.5, 0.0],
		                                           center: [10.0, 10.0, 2.0, 1.0],
		                                           color: [0.9, 0.9, 0.8, 1.0],
	                                           }];

	let mut elapsed = 0.0f32;
	let mut frame_count = 0u32;
	let mut start = SystemTime::now();
	let mut smooth: Smooth<f32> = Smooth::new(120);
	let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
	'main: loop {

		for event in window.poll_events() {
			match event {
				e @ glutin::Event::MouseMoved(_, _) |
				e @ glutin::Event::MouseInput(_, _) => app.mouse_input(e),

				glutin::Event::Resized(new_width, new_height) => {
					gfx_window_glutin::update_views(&window, &mut main_color, &mut main_depth);
					app.on_resize(new_width, new_height);
				}
				glutin::Event::Closed => break 'main,
				_ => {}
			}
		}

		let camera = render::Camera::ortho(cgmath::Point2::new(0.0f32, 0.0),
		                                   app.viewport.scale,
		                                   app.viewport.ratio);

		renderer.setup(&mut encoder, &camera, &lights);

		// update and measure
		let (frame_time, smooth_frame_time, fps) = match start.elapsed() {
			Ok(dt) => {
				let frame = (dt.as_secs() as f32) + (dt.subsec_nanos() as f32) * 1e-9;
				let smoothed = smooth.smooth(frame);
				app.update(smoothed);
				elapsed += frame;
				start = SystemTime::now();
				(frame, smoothed, 1.0 / smoothed)
			}
			Err(_) => (-1.0, -1.0, -1.0),
		};
		frame_count += 1;


		// draw a frame
		renderer.begin_frame(&mut encoder, &main_color, &main_depth);

		// draw the box2d bodies
		app.render(&renderer,
		           &mut encoder,
		           &vertex_buffer,
		           &slice,
		           &main_color,
		           &main_depth);

		// draw some debug text on screen
		renderer.draw_text(&mut encoder,
		                   &mut text_renderer,
		                   &format!("F: {} E: {:.3} FT: {:.2} SFT: {:.2} FPS: {:.1}",
		                            frame_count,
		                            elapsed,
		                            frame_time * 1000.0,
		                            smooth_frame_time * 1000.0,
		                            fps),
		                   [10, 10],
		                   [1.0; 4],
		                   &main_color);

		// push the commands
		renderer.end_frame(&mut encoder, &mut device);

		window.swap_buffers().unwrap();
		renderer.cleanup(&mut device);
	}

}
