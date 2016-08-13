mod app;
mod core;
mod frontend;
mod backend;

#[macro_use]
extern crate log;
extern crate simplelog;

#[macro_use]
extern crate bitflags;
extern crate cgmath;

extern crate wrapped2d;

#[macro_use]
extern crate gfx;
extern crate gfx_device_gl;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate genmesh;
extern crate piston;

extern crate rand;
extern crate num;
extern crate gfx_text;

fn main() {
	app::run();
}
