mod render;
mod app;

extern crate rand;
extern crate num;
extern crate gfx_text;

#[macro_use]
extern crate log;
extern crate simplelog;

extern crate cgmath;

extern crate wrapped2d;

#[macro_use]
extern crate gfx;
extern crate gfx_device_gl;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate genmesh;
extern crate piston;

use render::Renderer;

fn main() {
	const WIDTH: u32 = 1280;
	const HEIGHT: u32 = 720;

	let builder = glutin::WindowBuilder::new()
		              .with_title("Box2d + GFX".to_string())
		              .with_dimensions(WIDTH, HEIGHT)
		              .with_vsync();

	let (window,
	     mut device,
	     mut factory,
	     mut frame_buffer,
	     mut depth_buffer) = gfx_window_glutin::init::<render::ColorFormat, render::DepthFormat>(builder);

	let (w, h, _, _) = frame_buffer.get_dimensions();

	let mut encoder = factory.create_command_buffer().into();

	let renderer = &mut render::ForwardRenderer::new(&mut factory, &mut encoder, &frame_buffer, &depth_buffer);

	// Create a new game and run it.
	let mut app = app::App::new(w as u32, h as u32, 50.0);

	'main: loop {

		for event in window.poll_events() {
			match event {
				e @ glutin::Event::MouseMoved(_, _) |
				e @ glutin::Event::MouseInput(_, _) => app.on_mouse_input(e),
				e @ glutin::Event::KeyboardInput(_, _, _) => app.on_keyboard_input(e),

				glutin::Event::Resized(new_width, new_height) => {
					gfx_window_glutin::update_views(&window, &mut frame_buffer, &mut depth_buffer);
					app.on_resize(new_width, new_height);
				}
				glutin::Event::Closed => app.quit(),
				_ => {}
			}
		}

		if !app.is_running() {
			break 'main;
		}

		let camera = render::Camera::ortho(cgmath::Point2::new(0., 0.),
		                                   app.viewport.scale,
		                                   app.viewport.ratio);

		let environment = app.environment();

		renderer.setup(&camera, environment.background, environment.light);

		// update and measure
		let update_result = app.update();

		// draw a frame
		renderer.begin_frame();

		// draw the box2d bodies
		app.render(renderer);

		renderer.resolve_frame_buffer();

		if let Ok(r) = update_result {
			use render::Draw;
			// draw some debug text on screen
			renderer.draw_text(&format!("F: {} E: {:.3} FT: {:.2} SFT: {:.2} FPS: {:.1}",
			                            r.frame_count,
			                            r.frame_elapsed,
			                            r.frame_time * 1000.0,
			                            r.frame_time_smooth * 1000.0,
			                            r.fps),
			                   [10, 10],
			                   [1.0; 4]);
		}

		// push the commands
		renderer.end_frame(&mut device);

		window.swap_buffers().unwrap();
		renderer.cleanup(&mut device);
	}
}
