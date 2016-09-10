use std::path;
use frontend::render;
use frontend::input::EventMapper;
use frontend::render::Draw;
use frontend::render::Renderer;
use core::resource::filesystem::ResourceLoaderBuilder;
use core::math::Directional;
use app;
use app::ev::GlutinEventMapper;
use glutin;
use gfx_window_glutin;

pub fn main_loop() {
	const WIDTH: u32 = 1280;
	const HEIGHT: u32 = 720;

	let builder = glutin::WindowBuilder::new()
		.with_title("Box2d + GFX".to_string())
		.with_dimensions(WIDTH, HEIGHT)
		.with_vsync();

	let (window, mut device, mut factory, mut frame_buffer, mut depth_buffer) =
		gfx_window_glutin::init::<render::ColorFormat, render::DepthFormat>(builder);

	let (w, h, _, _) = frame_buffer.get_dimensions();

	let mut encoder = factory.create_command_buffer().into();

	let res = ResourceLoaderBuilder::new()
		.add(path::Path::new("resources"))
		.build();

	let renderer = &mut render::ForwardRenderer::new(&mut factory,
	                                                 &mut encoder,
	                                                 &res,
	                                                 &frame_buffer,
	                                                 &depth_buffer)
		.unwrap();
	let mapper = GlutinEventMapper::new();
	// Create a new game and run it.
	let mut app = app::App::new(w as u32, h as u32, 100.0, &res);

	app.init();

	'main: loop {
		for event in window.poll_events() {
			match event {
				glutin::Event::Resized(new_width, new_height) => {
					gfx_window_glutin::update_views(&window, &mut frame_buffer, &mut depth_buffer);
					renderer.resize_to(&frame_buffer, &depth_buffer).unwrap();
					app.on_resize(new_width, new_height);
				}
				glutin::Event::Closed => app.quit(),
				glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::F5)) => renderer.rebuild().unwrap(),
				e => {
					mapper.translate(&e).map(|i| app.on_input_event(&i));
				}
			}
		}

		if !app.is_running() {
			break 'main;
		}

		// update and measure
		let update_result = app.update();

		let camera = render::Camera::ortho(app.camera.position(),
		                                   app.viewport.scale,
		                                   app.viewport.ratio);

		let environment = app.environment();

		let light_positions = environment.light_positions.as_ref();
		renderer.setup_frame(&camera,
		                     environment.background_color,
		                     environment.light_color,
		                     light_positions);
		// draw a frame
		renderer.begin_frame();
		// draw the scene
		app.render(renderer);
		// post-render effects and tone mapping
		renderer.resolve_frame_buffer();

		let r = update_result;
		// draw some debug text on screen
		renderer.draw_text(&format!("F: {} E: {:.3} FT: {:.2} SFT: {:.2} FPS: {:.1}",
		                            r.frame_count,
		                            r.frame_elapsed,
		                            r.frame_time * 1000.0,
		                            r.frame_time_smooth * 1000.0,
		                            r.fps),
		                   [10, 10],
		                   [1.0; 4]);

		// push the commands
		renderer.end_frame(&mut device);

		window.swap_buffers().unwrap();
		renderer.cleanup(&mut device);
	}
}
