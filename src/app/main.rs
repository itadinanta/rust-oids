use std::path;
use frontend::render;
use frontend::input::EventMapper;
use frontend::render::formats;
use frontend::render::Draw;
use frontend::render::Renderer;
use frontend::audio;

use portaudio as pa;

use core::resource::filesystem::ResourceLoaderBuilder;
use core::math::Directional;
use app;
use app::ev::GlutinEventMapper;
use glutin;
use glutin::WindowEvent;
use glutin::VirtualKeyCode;
use glutin::KeyboardInput;
use glutin::GlContext;
use gfx_window_glutin;

pub fn main_loop(minion_gene_pool: &str) {
	const WIDTH: u32 = 1024;
	const HEIGHT: u32 = 1024;

	let mut events_loop = glutin::EventsLoop::new();

	let builder = glutin::WindowBuilder::new()
		.with_title("Rust-oids".to_string())
		.with_dimensions(WIDTH, HEIGHT);

	let context_builder = glutin::ContextBuilder::new();

	let (window, mut device, mut factory, mut frame_buffer, mut depth_buffer) =
		gfx_window_glutin::init::<formats::ScreenColorFormat, formats::ScreenDepthFormat>(
			builder,
			context_builder,
			&events_loop,
		);

	let (w, h, _, _) = frame_buffer.get_dimensions();

	let mut encoder = factory.create_command_buffer().into();

	let res = ResourceLoaderBuilder::new()
		.add(path::Path::new("resources"))
		.build();

	let renderer = &mut render::ForwardRenderer::new(&mut factory, &mut encoder, &res, &frame_buffer).unwrap();
	let mapper = GlutinEventMapper::new();
	// Create a new game and run it.
	let mut app = app::App::new(w as u32, h as u32, 100.0, &res, minion_gene_pool);

	let audio = audio::PortaudioSoundSystem::new();

	match audio.init_status  {
		pa::Error::NoError => println!("Success initializing portaudio"),
		failure => println!("Failure initializing portaudio: {:?}", failure),
	}

	app.init();

	'main: loop {
		events_loop.poll_events(|event| match event {
			glutin::Event::WindowEvent { event, .. } => {
				match event {
					WindowEvent::Resized(new_width, new_height) => {
						gfx_window_glutin::update_views(&window, &mut frame_buffer, &mut depth_buffer);
						renderer.resize_to(&frame_buffer).unwrap();
						app.on_resize(new_width, new_height);
					}
					WindowEvent::Closed => app.quit(),
					WindowEvent::KeyboardInput {
						input: KeyboardInput { virtual_keycode: Some(VirtualKeyCode::F5), .. }, ..
					} => renderer.rebuild().unwrap(),
					e => {
						mapper.translate(&e).map(|i| app.on_input_event(&i));
					}
				}
			}
			_ => {}
		});

		if !app.is_running() {
			break 'main;
		}

		// update and measure
		let update_result = app.update();

		app.audio_events();

		let camera = render::Camera::ortho(
			app.camera.position(),
			app.viewport.scale,
			app.viewport.ratio,
		);

		let environment = app.environment();

		let light_positions = environment.light_positions.as_ref();
		renderer.setup_frame(
			&camera,
			environment.background_color,
			environment.light_color,
			light_positions,
		);
		// draw a frame
		renderer.begin_frame();
		// draw the scene
		app.render(renderer);
		// post-render effects and tone mapping
		renderer.resolve_frame_buffer();

		let r = update_result;
		// draw some debug text on screen
		renderer.draw_text(
			&format!(
				"F: {} E: {:.3} FT: {:.2} SFT: {:.2} FPS: {:.1} P: {} E: {}",
				r.frame_count,
				r.frame_elapsed,
				r.frame_time * 1000.0,
				r.frame_time_smooth * 1000.0,
				r.fps,
				r.population,
				r.extinctions
			),
			[10, 10],
			[1.0; 4],
		);

		// push the commands
		renderer.end_frame(&mut device);

		window.swap_buffers().unwrap();
		renderer.cleanup(&mut device);
	}
}
