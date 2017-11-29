use std::path;
use std::io;

use frontend::render;
use frontend::input::EventMapper;
use frontend::render::{formats, Draw, Renderer};
use frontend::audio::{self, SoundSystem};
use frontend::ui;


use conrod;

use core::resource::filesystem::ResourceLoaderBuilder;
use core::math::Directional;
use core::clock::{Seconds, SecondsValue, Timer, Hourglass, SystemTimer};
use core::resource::ResourceLoader;
use ctrlc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use app;
use app::ev::GlutinEventMapper;
use glutin::{self, WindowEvent, VirtualKeyCode, KeyboardInput, GlContext};
use gfx_window_glutin;

pub fn main_loop(minion_gene_pool: &str, fullscreen: Option<usize>, width: Option<u32>, height: Option<u32>) {
	const WIDTH: u32 = 1024;
	const HEIGHT: u32 = 1024;

	let mut events_loop = glutin::EventsLoop::new();

	let builder = glutin::WindowBuilder::new()
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
	let mut ui_encoder = factory.create_command_buffer().into();

	let res = ResourceLoaderBuilder::new()
		.add(path::Path::new("resources"))
		.build();

	let renderer = &mut render::ForwardRenderer::new(&mut factory, &mut encoder, &res, &frame_buffer).unwrap();
	let mapper = GlutinEventMapper::new();
	// Create a new game and run it.
	let mut app = app::App::new(w as u32, h as u32, 100.0, &res, minion_gene_pool);

	let mut ui_renderer = ui::conrod_gfx::Renderer::new(&mut factory, &frame_buffer, window.hidpi_factor() as f64).unwrap();
	let mut ui = conrod::UiBuilder::new([w as f64, h as f64]).theme(ui::theme::theme()).build();
	let ui_image_map = conrod::image::Map::new();

	impl From<conrod::text::font::Error> for ui::Error {
		fn from(_: conrod::text::font::Error) -> ui::Error {
			ui::Error::FontLoader
		}
	}

	impl From<io::Error> for ui::Error {
		fn from(_: io::Error) -> ui::Error {
			ui::Error::ResourceLoader
		}
	}

	fn load_font<R>(res: &R, map: &mut conrod::text::font::Map, key: &str) ->
	Result<conrod::text::font::Id, ui::Error>
		where R: ResourceLoader<u8> {
		let font_bytes = res.load(key)?;
		let font_collection = conrod::text::FontCollection::from_bytes(font_bytes);
		let default_font = font_collection.into_font().ok_or(ui::Error::FontLoader)?;
		let id = map.insert(default_font);
		Ok(id)
	}

	let font_freesans = load_font(&res, &mut ui.fonts, "fonts/FreeSans.ttf")
		.expect("Could not find default font");

	let audio = audio::ThreadedSoundSystem::new().expect("Failure in audio initialization");
	let mut audio_alert_player = audio::ThreadedAlertPlayer::new(audio);
	app.init();

	'main: loop {
		events_loop.poll_events(|event| {
			if app.has_ui_overlay() {
				if let Some(event) = conrod::backend::winit::convert_event(event.clone(), window.window()) {
					ui.handle_event(event);
				}
			}

			match event {
				glutin::Event::WindowEvent { event, .. } => {
					match event {
						WindowEvent::Resized(new_width, new_height) => {
							gfx_window_glutin::update_views(&window, &mut frame_buffer, &mut depth_buffer);
							renderer.resize_to(&frame_buffer).unwrap();
							app.on_resize(new_width, new_height);
							ui = conrod::UiBuilder::new([new_width as f64, new_height as f64]).theme(ui::theme::theme()).build();
//                            ui.handle_event(conrod::event::Input::Resize(new_width, new_height))
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

		app.play_alerts(&mut audio_alert_player);
		app.play_interactions(&mut audio_alert_player);

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
			use conrod::{self, widget, Colorable, Positionable, Sizeable, Widget};
			widget_ids!(struct Ids { text, canvas, rounded_rectangle });
			let ids = Ids::new(ui.widget_id_generator());

			let window_id = ui.window.clone();
			let wui = &mut ui.set_widgets();
			// "Hello World!" in the middle of the screen.

			widget::Canvas::new()
				.pad(10.0)
				.color(conrod::color::CHARCOAL.alpha(0.4))
				.middle_of(window_id)
				.scroll_kids_vertically()
				.set(ids.canvas, wui);

//			widget::RoundedRectangle::fill([200.0, 100.0], 5.0)
//				.color(conrod::color::BLACK)
//				.w(wui.w_of(ids.canvas).unwrap_or_default())
//				.middle_of(ids.canvas)
//				.set(ids.rounded_rectangle, wui);

			widget::Text::new("Hello World!")
				.middle_of(ids.canvas)
				.wh([100.0, 100.0])
				.color(conrod::color::WHITE)
				.font_id(font_freesans)
				.font_size(32)
				.set(ids.text, wui);

			let primitives = wui.draw();

			let dims = (app.viewport.width as f32 * window.hidpi_factor(),
						app.viewport.height as f32 * window.hidpi_factor());

			ui_renderer.fill(&mut ui_encoder, dims, primitives, &ui_image_map);
			ui_renderer.draw(&mut factory, &mut ui_encoder, &ui_image_map);
		}

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
		ui_encoder.flush(&mut device);

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
	let mut headless_alert_player = ui::NullAlertPlayer::new();
	app.init();

	let running = Arc::new(AtomicBool::new(true));
	let r = running.clone();

	ctrlc::set_handler(move || {
		r.store(false, Ordering::SeqCst);
	}).expect("Error setting Ctrl-C handler");

	let wall_clock = SystemTimer::new().shared();
	let mut output_hourglass = Hourglass::new(wall_clock.clone(), Seconds::new(5.0));
	let mut save_hourglass = Hourglass::new(wall_clock.clone(), Seconds::new(300.0));

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
		let simulation_update = app.simulate(Seconds::new(FRAME_SIMULATION_LENGTH));
		if save_hourglass.flip_if_expired() {
			app.dump_to_file();
		}

		app.play_alerts(&mut headless_alert_player);
		if output_hourglass.flip_if_expired() {
			info!(
				"C: {} E: {:.3} FT: {:.2} P: {} E: {}",
				simulation_update.count,
				simulation_update.elapsed,
				simulation_update.dt,
				simulation_update.population,
				simulation_update.extinctions
			)
		}
	}
}
