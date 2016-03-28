extern crate piston;
extern crate graphics;
extern crate glutin_window;
extern crate opengl_graphics;
extern crate box2d;

use piston::window::WindowSettings;
use piston::event_loop::*;
use piston::input::*;
use piston::input::Input::*;
use glutin_window::GlutinWindow as Window;
use opengl_graphics::{GlGraphics, OpenGL};
use box2d::b2;

pub struct Viewport {
    width: u32,
    height: u32,
    scale: f32,
}

pub struct InputState {
    mouse_position: b2::Vec2,
}

pub struct App {
    gl: GlGraphics,
    input_state: InputState,
    viewport: Viewport,
    world: b2::World,
}

impl App {
    fn onClick(&mut self, btn: MouseButton, pos: b2::Vec2) {
    
    
    
    }

    fn input(&mut self, i: &Input) {
        match *i {
            Input::Press(Button::Mouse(b)) => {
            	let pos = self.input_state.mouse_position;
                self.onClick(b, pos);
            }
            Move(Motion::MouseCursor(x, y)) => {
                let ref viewport = self.viewport;
                let tx = (x as f32 - (viewport.width as f32 * 0.5)) / viewport.scale;
                let ty = ((viewport.height as f32 * 0.5) - y as f32)  / viewport.scale;
                self.input_state.mouse_position = b2::Vec2 { x: tx, y: ty };
            }
            Resize(w, h) => {
                self.viewport.width = w;
                self.viewport.height = h;
                self.viewport.scale = (w as f32) / 100.0;
            }
            _ => (),
        }
    }

    fn render(&mut self, args: &RenderArgs) {
        use graphics::*;

        const WHITE: [f32; 4] = [1.0; 4];
        const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
        const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
        const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

        let ref world = self.world;
        let ref viewport = self.viewport;

        self.gl.draw(args.viewport(), |c, g| {
            clear(WHITE, g);

            for (_, b) in world.bodies() {
                let body = b.borrow();
                let position = (*body).position();
                let angle = (*body).angle();

                let transform = c.transform // transform compose right to left
                                 .trans((viewport.width as f64 * 0.5), (viewport.height as f64 * 0.5))
                                 .scale(viewport.scale as f64, -viewport.scale as f64)
                                 .trans(position.x as f64, position.y as f64)
                                 .rot_deg(angle as f64);

                for (_, f) in body.fixtures() {
                    let fixture = f.borrow();
                    let shape = (*fixture).shape();

                    match *shape {
                        b2::UnknownShape::Circle(ref s) => {
                            let p = s.position();
                            let r = s.radius() as f64;
                            let extent = rectangle::square(p.x as f64 - r, p.y as f64 - r, r * 2.0);
                            Ellipse::new(RED).draw(extent, default_draw_state(), transform, g);
                        }
                        b2::UnknownShape::Polygon(ref s) => {
                            let n = s.vertex_count();
                            let mut v = Vec::with_capacity(n as usize);
                            for i in 0..n {
                                let vertex = s.vertex(i);
                                v.push([vertex.x as f64, vertex.y as f64]);
                            }
                            Polygon::new(BLACK)
                                .draw(v.as_slice(), default_draw_state(), transform, g);
                        }
                        _ => (),
                    }
                }
            }
        });
    }

    fn update(&mut self, args: &UpdateArgs) {
        // Rotate 2 radians per second.
        self.world.step(args.dt as f32, 8, 3);
    }
}

fn new_world() -> b2::World {
    let mut world = b2::World::new(&b2::Vec2 { x: 0.0, y: -9.8 });

    let mut b_def = b2::BodyDef::new();
    b_def.body_type = b2::BodyType::Static;
    b_def.position = b2::Vec2 { x: 0., y: -20. };

    let mut ground_box = b2::PolygonShape::new();
    ground_box.set_as_box(20., 1.);

    let ground_handle = world.create_body(&b_def);
    world.get_body_mut(ground_handle)
         .create_fast_fixture(&ground_box, 0.);

    let mut b_def = b2::BodyDef::new();
    b_def.body_type = b2::BodyType::Dynamic;
    b_def.position = b2::Vec2 { x: 0., y: 0. };

    let mut cube_shape = b2::PolygonShape::new();
    cube_shape.set_as_box(1., 1.);

    let mut circle_shape = b2::CircleShape::new();
    circle_shape.set_radius(1.);

    let mut f_def = b2::FixtureDef::new();
    f_def.density = 1.;
    f_def.restitution = 0.2;
    f_def.friction = 0.3;

    b_def.position.x += 0.5;
    if b_def.position.x > 20. {
        b_def.position.x = -20.;
    }
    let handle = world.create_body(&b_def);
    world.get_body_mut(handle)
         .create_fixture(&circle_shape, &mut f_def);

    return world;
}

fn main() {
	const WIDTH: u32 = 1920;
	const HEIGHT: u32 = 1080;
    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V3_2;
    // Create an Glutin window.
    let mut window: Window = WindowSettings::new("box2d",
                                                 [WIDTH, HEIGHT])
                                 .opengl(opengl)
                                 .exit_on_esc(true)
                                 .build()
                                 .unwrap();

    // Create a new game and run it.
    let mut app = App {
        gl: GlGraphics::new(opengl),
        input_state: InputState { mouse_position: b2::Vec2 { x: 0.0, y: 0.0 } },
        viewport: Viewport {
            width: WIDTH,
            height: HEIGHT,
            scale: (WIDTH as f32) / 100.0,
        },
        world: new_world(),
    };




    let mut events = window.events();
    while let Some(e) = events.next(&mut window) {
        match e {
            Event::Render(args) => app.render(&args),
            Event::Update(args) => app.update(&args),
            Event::Input(input) => app.input(&input),
            _ => (),
        }
    }
}
