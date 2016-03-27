extern crate piston;
extern crate graphics;
extern crate glutin_window;
extern crate opengl_graphics;
extern crate box2d;

use piston::window::WindowSettings;
use piston::event_loop::*;
use piston::input::*;
use glutin_window::GlutinWindow as Window;
use opengl_graphics::{GlGraphics, OpenGL};
use box2d::b2;

pub struct App {
    gl: GlGraphics,
    world: b2::World,
}

impl App {
    fn render(&mut self, args: &RenderArgs) {
        use graphics::*;

        const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
        const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
        const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

        let ref world = self.world;
        self.gl.draw(args.viewport(), |c, gl| {
            clear(GREEN, gl);

            for (_, b) in world.bodies() {
                let body = b.borrow();
                let position = (*body).position();
                let angle = (*body).angle();

                let transform = c.transform
                                 .trans((args.width as f64 * 0.5), (args.height as f64 * 0.5))
                                 .scale(2.0, 2.0)
                                 .trans(position.x as f64, position.y as f64)
                                 .rot_deg(angle as f64);

                let square = rectangle::square(0.0, 0.0, 50.0);

                for (_, f) in body.fixtures() {
                    let fixture = f.borrow();
                    let shape = (*fixture).shape();

                    match *shape {
                        b2::UnknownShape::Circle(ref s) => {
                            let p = s.position();
                            let r = s.radius() as f64;
                            let extent = rectangle::square(p.x as f64 - r / 2.0,
                                                           p.y as f64 - r / 2.0,
                                                           r);
                            Ellipse::new(RED).draw(extent, default_draw_state(), transform, gl);
                        }
                        b2::UnknownShape::Polygon(ref s) => {
                            rectangle(BLACK, square, transform, gl);
                        }
                        _ => (),
                    }

                }
            }
        });
        //         let square = rectangle::square(0.0, 0.0, 50.0);
        // let rotation = self.rotation;
        // let (x, y) = ((args.width / 2) as f64, (args.height / 2) as f64);

        // let transform = c.transform
        // .trans(x, y)
        // .rot_rad(rotation)
        // .trans(-25.0, -25.0);
        //
        // Draw a box rotating around the middle of the screen.
        //
        // });
        //
    }

    fn update(&mut self, args: &UpdateArgs) {
        // Rotate 2 radians per second.
        self.world.step(args.dt as f32, 8, 3);
    }
}

fn new_world() -> b2::World {
    let mut world = b2::World::new(&b2::Vec2 { x: 0.0, y: 2.0 });

    let mut b_def = b2::BodyDef::new();
    b_def.body_type = b2::BodyType::Static;
    b_def.position = b2::Vec2 { x: 0., y: -10. };

    let mut ground_box = b2::PolygonShape::new();
    ground_box.set_as_box(20., 1.);

    let ground_handle = world.create_body(&b_def);
    world.get_body_mut(ground_handle)
         .create_fast_fixture(&ground_box, 0.);

    let mut b_def = b2::BodyDef::new();
    b_def.body_type = b2::BodyType::Dynamic;
    b_def.position = b2::Vec2 { x: -20., y: 20. };

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
    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V3_2;

    // Create an Glutin window.
    let mut window: Window = WindowSettings::new("box2d", [200, 200])
                                 .opengl(opengl)
                                 .exit_on_esc(true)
                                 .build()
                                 .unwrap();


    // Create a new game and run it.
    let mut app = App {
        gl: GlGraphics::new(opengl),
        world: new_world(),
    };

    let mut events = window.events();
    while let Some(e) = events.next(&mut window) {
        if let Some(r) = e.render_args() {
            app.render(&r);
        }

        if let Some(u) = e.update_args() {
            app.update(&u);
        }
    }
}
