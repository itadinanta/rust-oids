use frontend::audio::{self, SoundSystem};
use frontend::gfx_window_glutin;
use frontend::input::EventMapper;
use frontend::input::GamepadEventLoop;
use frontend::render;
use frontend::render::{formats, Overlay, Renderer};
use frontend::ui;
use std::path;

use conrod;

use core::clock::{seconds, Hourglass, SecondsValue, SystemTimer};
use core::math::Directional;
use core::resource::filesystem::ResourceLoader;
use core::resource::filesystem::ResourceLoaderBuilder;
use ctrlc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use app;
use app::capture::Capture;
use app::constants::*;
use glutin;
use glutin::GlContext;
use winit::{self, KeyboardInput, VirtualKeyCode, WindowEvent};

pub fn make_resource_loader(config_home: &path::Path) -> ResourceLoader {
	ResourceLoaderBuilder::new()
		.add(path::Path::new(CONFIG_DIR_RESOURCES))
		.add(config_home.join(CONFIG_DIR_RESOURCES).as_path())
		.add(config_home.join(CONFIG_DIR_SAVED_STATE).as_path())
		.add(
			path::Path::new("/usr/local/share/rust-oids")
				.join(CONFIG_DIR_RESOURCES)
				.as_path(),
		).add(
			path::Path::new("/usr/share/rust-oids")
				.join(CONFIG_DIR_RESOURCES)
				.as_path(),
		).build()
}

pub fn main_loop(
	minion_gene_pool: &str,
	config_home: path::PathBuf,
	world_file: Option<path::PathBuf>,
	fullscreen: Option<usize>,
	width: Option<u32>,
	height: Option<u32>,
	audio_device: Option<usize>,
)
{
	let mut events_loop = winit::EventsLoop::new();
	let mut gamepad = GamepadEventLoop::new();

	let builder = winit::WindowBuilder::new().with_title("Rust-oids".to_string());
	let builder = if let Some(monitor_index) = fullscreen {
		let monitor = events_loop
			.get_available_monitors()
			.nth(monitor_index)
			.expect("Please enter a valid monitor ID");
		println!("Using {:?}", monitor.get_name());
		builder.with_fullscreen(Some(monitor))
	} else {
		builder.with_dimensions(
			width.unwrap_or(DEFAULT_WINDOW_WIDTH),
			height.unwrap_or(DEFAULT_WINDOW_HEIGHT),
		)
	};
	let context_builder = glutin::ContextBuilder::new().with_vsync(true);

	let (window, mut device, mut factory, mut frame_buffer, mut depth_buffer) =
		gfx_window_glutin::init::<formats::ScreenColorFormat, formats::ScreenDepthFormat>(
			builder,
			context_builder,
			&events_loop,
		);
	let (w, h, _, _) = frame_buffer.get_dimensions();
	let mut capture = Capture::init(&window);

	let mut encoder = factory.create_command_buffer().into();

	let res = make_resource_loader(&config_home);

	let renderer = &mut render::ForwardRenderer::new(&mut factory, &mut encoder, &res, &frame_buffer).unwrap();
	let mapper = app::WinitEventMapper::new();

	// Create a new game and run it.
	let mut app = app::App::new(
		w as u32,
		h as u32,
		VIEW_SCALE_BASE,
		config_home,
		&res,
		minion_gene_pool,
		world_file,
	);

	let mut ui = ui::conrod_ui::Ui::new(&res, &mut factory, &frame_buffer, window.hidpi_factor() as f64)
		.expect("Unable to create UI");

	let audio = audio::ThreadedSoundSystem::new(audio_device).expect("Failure in audio initialization");
	let mut no_audio = ui::NullAlertPlayer::new();
	let mut audio_alert_player = audio::ThreadedAlertPlayer::new(audio);
	app.init(app::SystemMode::Interactive);

	'main: loop {
		gamepad.poll_events(|event| app.on_input_event(&event));

		events_loop.poll_events(|event| {
			if app.has_ui_overlay() {
				if let Some(event) = conrod::backend::winit::convert_event(event.clone(), window.window()) {
					ui.push_event(event);
				}
			}
			match event {
				winit::Event::WindowEvent { event, .. } => match event {
					WindowEvent::Resized(new_width, new_height) => {
						gfx_window_glutin::update_views(&window, &mut frame_buffer, &mut depth_buffer);
						renderer.resize_to(&frame_buffer).expect("Unable to resize window");
						ui.resize_to(&frame_buffer).expect("Unable to resize window");
						app.on_resize(new_width, new_height);
					}
					WindowEvent::Closed => app.quit(),
					WindowEvent::KeyboardInput {
						input: KeyboardInput {
							virtual_keycode: Some(VirtualKeyCode::F5),
							..
						},
						..
					} => renderer.rebuild().unwrap(),
					e => {
						mapper.translate(&e).map(|i| app.on_input_event(&i));
					}
				},
				_ => {}
			}
		});

		capture.enable(app.is_capturing());

		if !app.is_running() {
			capture.stop();
			app.save_world_to_file();
			break 'main;
		}

		let speed_factor = app.speed_factors.get();
		let frame_update = if capture.enabled() || speed_factor > 5.0 {
			// forces 60Hz simulation for frame capture and fast forward
			app.update_with_quantum(Some(FRAME_TIME_TARGET))
		} else {
			// update and measure, let the app determine the appropriate frame length
			app.update()
		};

		let camera = render::Camera::ortho(app.camera.position(), app.viewport.scale, app.viewport.ratio);

		let environment = app.environment();

		renderer.setup_frame(&camera, environment.background_color, &environment.lights);
		// draw a frame
		renderer.begin_frame();
		// draw the scene
		app.paint(renderer);
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

		if speed_factor < 10.0 {
			app.play_alerts(&mut audio_alert_player);
		} else {
			app.play_alerts(&mut no_audio);
		};

		// push the commands
		renderer.end_frame(&mut device);
		capture.screen_grab();

		window.swap_buffers().expect("swap_buffers() failed");
		renderer.cleanup(&mut device);
	}
}

pub fn main_loop_headless(minion_gene_pool: &str, config_home: path::PathBuf, world_file: Option<path::PathBuf>) {
	const WIDTH: u32 = 1024;
	const HEIGHT: u32 = 1024;
	let res = make_resource_loader(&config_home);

	let mut app = app::App::new(
		WIDTH,
		HEIGHT,
		VIEW_SCALE_BASE,
		config_home,
		&res,
		minion_gene_pool,
		world_file,
	);
	let mut no_audio = ui::NullAlertPlayer::new();
	app.init(app::SystemMode::Batch);

	let running = Arc::new(AtomicBool::new(true));
	let r = running.clone();

	ctrlc::set_handler(move || {
		r.store(false, Ordering::SeqCst);
	}).expect("Error setting Ctrl-C handler");

	let wall_clock = SystemTimer::new();
	let mut output_hourglass = Hourglass::new(seconds(LOG_INTERVAL), &wall_clock);
	let mut save_hourglass = Hourglass::new(seconds(SAVE_INTERVAL), &wall_clock);

	const FRAME_SIMULATION_LENGTH: SecondsValue = FRAME_TIME_TARGET;
	'main: loop {
		if !app.is_running() {
			break 'main;
		}

		if !running.load(Ordering::SeqCst) {
			eprintln!("Interrupted, exiting");
			app.save_world_to_file();
			break 'main;
		}
		// update and measure
		let simulation_update = app.simulate(seconds(FRAME_SIMULATION_LENGTH));
		if save_hourglass.flip_if_expired(&wall_clock) {
			app.save_world_to_file();
		}

		app.play_alerts(&mut no_audio);
		if output_hourglass.flip_if_expired(&wall_clock) {
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
