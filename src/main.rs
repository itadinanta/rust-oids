mod shaders;

extern crate piston;
extern crate graphics;
extern crate wrapped2d;
extern crate rand;
extern crate cgmath;

#[macro_use]
extern crate gfx;
extern crate gfx_device_gl;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate genmesh;

use std::time::SystemTime;
use rand::Rng;

use piston::window::WindowSettings;
use piston::event_loop::*;
use piston::input::*;
use piston::input::Input::*;

use gfx::Device;
use gfx::traits::FactoryExt;

use cgmath::{Point3, Vector3, Matrix4, EuclideanVector};
use cgmath::{Transform, AffineMatrix3};
use genmesh::generators::SphereUV;
use genmesh::{Triangulate, MapToVertices, Vertices};

use wrapped2d::b2;
use std::f64::consts;

pub struct Viewport {
	width: u32,
	height: u32,
	scale: f32
}

pub struct InputState {
	left_button_pressed: bool,
	mouse_position: b2::Vec2
}

pub type ColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;


gfx_defines!{
    vertex Vertex {
        pos: [f32; 2] = "a_Pos",
        color: [f32; 3] = "a_Color",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        out: gfx::RenderTarget<ColorFormat> = "Target0",
    }
}

const TRIANGLE: [Vertex; 3] = [Vertex {
	                               pos: [-0.5, -0.5],
	                               color: [1.0, 0.0, 0.0]
                               },
                               Vertex {
	                               pos: [0.5, -0.5],
	                               color: [0.0, 1.0, 0.0]
                               },
                               Vertex {
	                               pos: [0.0, 0.5],
	                               color: [0.0, 0.0, 1.0]
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
	input_state: InputState,
	world: b2::World
}

impl App {
	fn on_click(&mut self, btn: MouseButton, pos: b2::Vec2) {
		match btn {
			MouseButton::Left => {
				self.input_state.left_button_pressed = true;
				new_ball(&mut self.world, pos);
			}
			_ => (),
		}
	}

	fn on_drag(&mut self, pos: b2::Vec2) {
		new_ball(&mut self.world, pos);
	}

	fn on_release(&mut self, btn: MouseButton, _: b2::Vec2) {
		match btn {
			MouseButton::Left => {
				self.input_state.left_button_pressed = false;
			}
			_ => (),
		}
	}

	fn input(&mut self, i: &Input) {
		match *i {
			Release(Button::Mouse(b)) => {
				let pos = self.input_state.mouse_position;
				self.on_release(b, pos);
			}
			Press(Button::Mouse(b)) => {
				let pos = self.input_state.mouse_position;
				self.on_click(b, pos);
			}
			Move(Motion::MouseCursor(x, y)) => {
				fn transform_pos(viewport: &Viewport, x: f64, y: f64) -> b2::Vec2 {
					let viewport = &viewport;
					let tx = (x as f32 - (viewport.width as f32 * 0.5)) / viewport.scale;
					let ty = ((viewport.height as f32 * 0.5) - y as f32) / viewport.scale;
					return b2::Vec2 { x: tx, y: ty };
				}
			}
			_ => (),
		}
	}

	fn render(&mut self, args: &RenderArgs) {
		use graphics::*;

		// self.gl.draw(args.viewport(), |c, g| {
		// clear(WHITE, g);
		//
		// for (_, b) in world.bodies() {
		// let body = b.borrow();
		// let position = (*body).position();
		// let angle = (*body).angle();
		//
		// let transform = c.transform // transform compose right to left
		// .trans((viewport.width as f64 * 0.5), (viewport.height as f64 * 0.5))
		// .scale(viewport.scale as f64, -viewport.scale as f64)
		// .trans(position.x as f64, position.y as f64)
		// .rot_rad(angle as f64);
		//
		// for (_, f) in body.fixtures() {
		// let fixture = f.borrow();
		// let shape = (*fixture).shape();
		// let density = (*fixture).density();
		//
		// match *shape {
		// b2::UnknownShape::Circle(ref s) => {
		// let p = s.position();
		// let r = s.radius() as f64;
		// let extent = rectangle::square(p.x as f64 - r, p.y as f64 - r, r * 2.0);
		// let colour = [1.0, 0.0, 0.0, density - 1.0];
		// Ellipse::new(colour).draw(extent, &DrawState::default(), transform, g);
		// }
		// b2::UnknownShape::Polygon(ref s) => {
		// let n = s.vertex_count();
		// let mut v = Vec::with_capacity(n as usize);
		// for i in 0..n {
		// let vertex = s.vertex(i);
		// v.push([vertex.x as f64, vertex.y as f64]);
		// }
		// Polygon::new(BLACK)
		// .draw(v.as_slice(), &DrawState::default(), transform, g);
		// }
		// _ => (),
		// }
		// }
		// }
		// });
		//
	}

	fn update(&mut self, dt: f32) {
		let world = &mut self.world;
		world.step(dt, 8, 3);
		const MAX_RADIUS: f32 = 5.0;
		let edge = 0.0f32; //-(self.viewport.height as f32) / 2.0 / self.viewport.scale - MAX_RADIUS;
		let mut v = Vec::new();
		for (h, b) in world.bodies() {
			let body = b.borrow();
			let position = (*body).position();
			if position.y < edge {
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
	b_def.position = b2::Vec2 { x: 0.0, y: -20.0 };

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

fn main() {
	const WIDTH: u32 = 1920;
	const HEIGHT: u32 = 1080;

	let builder = glutin::WindowBuilder::new()
		              .with_title("Amethyst Renderer Demo".to_string())
		              .with_dimensions(WIDTH, HEIGHT)
		              .with_vsync();

	let (window, mut device, mut factory, main_color, main_depth) =
		gfx_window_glutin::init::<ColorFormat, DepthFormat>(builder);
	let combuf = factory.create_command_buffer();
	let (mut w, mut h, _, _) = main_color.get_dimensions();

	// Create a new game and run it.
	let mut app = App {
		input_state: InputState {
			left_button_pressed: false,
			mouse_position: b2::Vec2 { x: 0.0, y: 0.0 }
		},
		world: new_world()
	};

	let mut start = SystemTime::now();
	let c = main_color;

	let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
	let pso = factory.create_pipeline_simple(&shaders::SIMPLE_VERTEX,
	                                         &shaders::SIMPLE_PIXEL,
	                                         pipe::new())
	                 .unwrap();

	const WHITE: [f32; 4] = [1.0; 4];
	const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

	let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&TRIANGLE, ());
	let data = pipe::Data {
		vbuf: vertex_buffer,
		out: c
	};

	'main: loop {
		// events
		for event in window.poll_events() {
			match event {
				// glutin::Event::Input(e) => app.input(e),
				glutin::Event::Closed => break 'main,
				_ => {}
			}
		}

		// update
		match start.elapsed() {
			Ok(dt) => app.update((dt.as_secs() as f32) + (dt.subsec_nanos() as f32) * 1e-9),
			Err(_) => {}
		}

		// draw a frame
		encoder.clear(&data.out, BLACK);
		encoder.draw(&slice, &pso, &data);
		encoder.flush(&mut device);
		window.swap_buffers().unwrap();
		device.cleanup();

		start = SystemTime::now();
	}
}
