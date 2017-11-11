use std::path;
use frontend::render;
use frontend::input::EventMapper;
use frontend::render::formats;
use frontend::render::Draw;
use frontend::render::Renderer;
use frontend::audio;
use frontend::audio::SoundSystem;

use core::resource::filesystem::ResourceLoaderBuilder;
use core::math::Directional;
use core::clock::Seconds;

use ctrlc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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

	let mut audio = audio::PortaudioSoundSystem::new();
	match &audio {
		&Ok(_) => println!("Success initializing portaudio"),
		&Err(msg) => println!("Failure initializing portaudio: {:?}", msg),
	}
	let mut audio = audio.unwrap();
	let mut audio_alert_player = audio::PortaudioAlertPlayer::new(audio);
	audio_alert_player.open().expect("Could not open audio player");
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

		// update and measure, let the app determine the appropriate frame length
		let frame_update = app.update();

		app.play_alerts(&mut audio_alert_player);

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

		// draw some debug text on screen
		renderer.draw_text(
			&format!(
				"F: {},{} E: {:.3} FT: {:.3},{:.3}(x{}) SFT: {:.3} FPS: {:.1} P: {} E: {}",
				frame_update.simulation.count,
				frame_update.count,
				frame_update.elapsed,
				frame_update.simulation.dt,
				frame_update.dt,
				frame_update.speed_factor,
				frame_update.duration_smooth,
				frame_update.fps,
				frame_update.simulation.population,
				frame_update.simulation.extinctions
			),
			[10, 10],
			[1.0; 4],
		);

		// push the commands
		renderer.end_frame(&mut device);

		window.swap_buffers().unwrap();
		renderer.cleanup(&mut device);
	};
	audio_alert_player.close().expect("Could not close audio player");
}

pub fn main_loop_headless(minion_gene_pool: &str) {
	const WIDTH: u32 = 1024;
	const HEIGHT: u32 = 1024;
	let res = ResourceLoaderBuilder::new()
		.add(path::Path::new("resources"))
		.build();

	let mut app = app::App::new(WIDTH, HEIGHT, 100.0, &res, minion_gene_pool);
	app.init();

	let running = Arc::new(AtomicBool::new(true));
	let r = running.clone();

	ctrlc::set_handler(move || {
		r.store(false, Ordering::SeqCst);
	}).expect("Error setting Ctrl-C handler");

	const FRAME_SIMULATION_LENGTH: f32 = 1.0f32 / 60.0f32;
	'main: loop {
		if !app.is_running() {
			break 'main;
		}

		if !running.load(Ordering::SeqCst) {
			eprintln!("Interrupted, exiting");
			break 'main;
		}
		// update and measure
		let simulation_update = app.simulate(Seconds::new(FRAME_SIMULATION_LENGTH));
		println!(
			"C: {} E: {:.3} FT: {:.2} P: {} E: {}",
			simulation_update.count,
			simulation_update.elapsed,
			simulation_update.dt,
			simulation_update.population,
			simulation_update.extinctions
		)
	}
}
