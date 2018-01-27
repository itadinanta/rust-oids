use std::path;

use frontend::render;
use frontend::input::EventMapper;
use frontend::input::GamepadEventLoop;
use frontend::render::{formats, Renderer, Overlay};
use frontend::audio::{self, SoundSystem};
use frontend::ui;

use conrod;

use core::resource::filesystem::ResourceLoaderBuilder;
use core::math::Directional;
use core::clock::{seconds, SecondsValue, Timer, Hourglass, SystemTimer};
use ctrlc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use app;
use winit::{self, WindowEvent, VirtualKeyCode, KeyboardInput};
use glutin;
use glutin::GlContext;
use gfx_window_glutin;

pub fn main_loop(minion_gene_pool: &str, fullscreen: Option<usize>, width: Option<u32>, height: Option<u32>) {
	const WIDTH: u32 = 1280;
	const HEIGHT: u32 = 1024;

	let mut events_loop = winit::EventsLoop::new();
	let mut gamepad = GamepadEventLoop::new();

	let builder = winit::WindowBuilder::new()
		.with_title("Rust-oids".to_string());
	let builder = if let Some(monitor_index) = fullscreen {
		let monitor = events_loop.get_available_monitors().nth(monitor_index).expect("Please enter a valid monitor ID");
		println!("Using {:?}", monitor.get_name());
		builder.with_fullscreen(Some(monitor))
	} else {
		builder.with_dimensions(width.unwrap_or(WIDTH), height.unwrap_or(HEIGHT))
	};
	let context_builder = glutin::ContextBuilder::new()
		.with_vsync(true);

	let (window,
		mut device,
		mut factory,
		mut frame_buffer,
		mut depth_buffer) =
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
	let mapper = app::WinitEventMapper::new();

	// Create a new game and run it.
	let mut app = app::App::new(w as u32, h as u32, 100.0, &res, minion_gene_pool);

	let mut ui = ui::conrod_ui::Ui::new(&res,
										&mut factory,
										&frame_buffer, window.hidpi_factor() as f64)
		.expect("Unable to create UI");

	let audio = audio::ThreadedSoundSystem::new().expect("Failure in audio initialization");
	let mut audio_alert_player = audio::ThreadedAlertPlayer::new(audio);
	app.init();

	'main: loop {
		gamepad.poll_events(|event| app.on_input_event(&event));

		events_loop.poll_events(|event| {
			if app.has_ui_overlay() {
				if let Some(event) = conrod::backend::winit::convert_event(event.clone(), window.window()) {
					ui.push_event(event);
				}
			}

			match event {
				winit::Event::WindowEvent { event, .. } => {
					match event {
						WindowEvent::Resized(new_width, new_height) => {
							gfx_window_glutin::update_views(&window, &mut frame_buffer, &mut depth_buffer);
							renderer.resize_to(&frame_buffer)
								.expect("Unable to resize window");
							ui.resize_to(&frame_buffer)
								.expect("Unable to resize window");
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
			}
		});

		if !app.is_running() {
			break 'main;
		}

		// update and measure, let the app determine the appropriate frame length
		let frame_update = app.update();

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

		if app.has_ui_overlay() {
			let screen = ui::Screen::Main(frame_update);
			renderer.overlay(|_, encoder| {
				ui.update_and_draw_screen(&screen, encoder);
			});
			ui.handle_events();

			for app_event in ui.drain_app_events() {
				app.interact(app_event)
			}
		}

		app.play_alerts(&mut audio_alert_player);
		app.play_interactions(&mut audio_alert_player);

		// push the commands
		renderer.end_frame(&mut device);

		window.swap_buffers().unwrap();
		renderer.cleanup(&mut device);
	};
}

pub fn main_loop_headless(minion_gene_pool: &str) {
	const WIDTH: u32 = 1024;
	const HEIGHT: u32 = 1024;
	let res = ResourceLoaderBuilder::new()
		.add(path::Path::new("resources"))
		.build();

	let mut app = app::App::new(WIDTH, HEIGHT, 100.0, &res, minion_gene_pool);
	let mut no_audio = ui::NullAlertPlayer::new();
	app.init();

	let running = Arc::new(AtomicBool::new(true));
	let r = running.clone();

	ctrlc::set_handler(move || {
		r.store(false, Ordering::SeqCst);
	}).expect("Error setting Ctrl-C handler");

	let wall_clock = SystemTimer::new().shared();
	let mut output_hourglass = Hourglass::new(wall_clock.clone(), seconds(5.0));
	let mut save_hourglass = Hourglass::new(wall_clock.clone(), seconds(300.0));

	const FRAME_SIMULATION_LENGTH: SecondsValue = 1.0 / 60.0;
	'main: loop {
		if !app.is_running() {
			break 'main;
		}

		if !running.load(Ordering::SeqCst) {
			eprintln!("Interrupted, exiting");
			app.dump_to_file();
			break 'main;
		}
// update and measure
		let simulation_update = app.simulate(seconds(FRAME_SIMULATION_LENGTH));
		if save_hourglass.flip_if_expired() {
			app.dump_to_file();
		}

		app.play_alerts(&mut no_audio);
		if output_hourglass.flip_if_expired() {
			info!(
				"C: {} E: {:.3} FT: {:.2} P: {} X: {}",
				simulation_update.count,
				simulation_update.elapsed,
				simulation_update.dt,
				simulation_update.population,
				simulation_update.extinctions
			)
		}
	}
}
