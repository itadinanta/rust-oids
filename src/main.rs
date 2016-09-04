mod app;
mod core;
mod frontend;
mod backend;

#[macro_use]
extern crate log;
extern crate simplelog;
extern crate chrono;

#[macro_use]
extern crate custom_derive;
#[macro_use]
extern crate enum_derive;

#[macro_use]
extern crate bitflags;
extern crate bit_set;
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
extern crate itertools;

#[macro_use]
extern crate enum_primitive;
extern crate gfx_text;

extern crate rustc_serialize as serialize;

fn main() {
	app::run();
}
