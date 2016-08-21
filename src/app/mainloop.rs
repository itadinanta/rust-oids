use frontend::render;
use frontend::input;
use frontend::input::Key;
use frontend::render::Draw;
use frontend::render::Renderer;
use core::math::Directional;
use app;
use cgmath;
use glutin;
use gfx_window_glutin;

fn translate(e: &glutin::Event) -> Option<input::Event> {
	fn keymap(vk: glutin::VirtualKeyCode) -> Option<input::Key> {
		macro_rules! glutin_map {
			[$($gkey:ident -> $ekey:ident),*] => (
				match vk {
					$(glutin::VirtualKeyCode::$gkey => Some(Key::$ekey)),
					*,
					_ => None,
				}
			)
		}
		glutin_map! [
			F1 -> F1,
			F2 -> F2,
			F3 -> F3,
			F4 -> F4,
			F5 -> F5,
			F6 -> F6,
			F7 -> F7,
			F8 -> F8,
			F9 -> F9,
			F10 -> F10,
			F11 -> F11,
			F12 -> F12,
			Home -> Home,
			Down -> Down,
			Up -> Down,
			Left -> Left,
			Right -> Right,
			A -> A,
			B -> B,
			C -> C,
			D -> D,
			E -> E,
			F -> F,
			G -> G,
			H -> H,
			I -> I,
			J -> J,
			K -> K,
			L -> L,
			M -> M,
			N -> N,
			O -> O,
			P -> P,
			Q -> Q,
			R -> R,
			S -> S,
			T -> T,
			U -> U,
			V -> V,
			W -> W,
			X -> X,
			Y -> Y,
			Z -> Z,
			Escape -> Esc
		]
	}
	match e {
		&glutin::Event::ReceivedCharacter(char) => {
			match char {
				_ => {
					println!("Key pressed {:?}", char);
					None
				}
			}
		}
		&glutin::Event::KeyboardInput(elementState, scancode, vk) => {
			let state = match elementState {
				glutin::ElementState::Pressed => input::State::Down,
				glutin::ElementState::Released => input::State::Up,
			};
			vk.and_then(|vk| keymap(vk)).and_then(|key| Some(input::Event::Key(state, key)))
		}
		_ => None,
	}
}



pub fn main_loop() {
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
	let mut app = app::App::new(w as u32, h as u32, 100.0);

	'main: loop {
		for event in window.poll_events() {
			match event {
				glutin::Event::Resized(new_width, new_height) => {
					gfx_window_glutin::update_views(&window, &mut frame_buffer, &mut depth_buffer);
					renderer.resize_to(&frame_buffer, &depth_buffer);
					app.on_resize(new_width, new_height);
				}
				glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::F5)) => renderer.rebuild(),
				e => {
					translate(&e).map(|i| app.on_input_event(&i));
				}
			}
		}

		if !app.is_running() {
			break 'main;
		}

		let camera = render::Camera::ortho(app.camera.position(),
		                                   app.viewport.scale,
		                                   app.viewport.ratio);

		let environment = app.environment();

		renderer.setup(&camera,
		               environment.background,
		               environment.light,
		               environment.light_position);

		// update and measure
		let update_result = app.update();

		// draw a frame
		renderer.begin_frame();

		// draw the scene
		app.render(renderer);

		// post-render effects and tone mapping
		renderer.resolve_frame_buffer();

		if let Ok(r) = update_result {
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
