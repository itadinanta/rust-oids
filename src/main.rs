mod app;
mod core;
mod frontend;
mod backend;

#[macro_use]
extern crate log;
extern crate log4rs;
extern crate chrono;
extern crate csv;

#[macro_use]
extern crate bitflags;
extern crate bit_set;
extern crate cgmath;

extern crate wrapped2d;

#[macro_use]
extern crate gfx;
extern crate gfx_device_gl;
extern crate gfx_window_glutin;
extern crate winit;
extern crate glutin;

extern crate portaudio;
extern crate pitch_calc;
extern crate sample;

extern crate rand;
extern crate num;
extern crate num_traits;
extern crate itertools;

#[cfg(profiler)]
extern crate cpuprofiler;

#[macro_use]
extern crate enum_primitive;
extern crate conrod;

extern crate getopts;
extern crate ctrlc;

extern crate gilrs;

#[cfg(unix)]
extern crate thread_priority;

extern crate rustc_serialize as serialize;

fn main() {
	use log4rs::config::*;
	use log4rs::append::console::*;
	use std::env;
	let args = env::args_os().collect::<Vec<_>>();

	let config = Config::builder()
		.appender(Appender::builder().build(
			"stdout".to_string(),
			Box::new(
				ConsoleAppender::builder().build(),
			),
		))
		.logger(Logger::builder().build(
			"gfx_device_gl".to_string(),
			log::LevelFilter::Error,
		))
		.logger(Logger::builder().build(
			"rust_oids".to_string(),
			log::LevelFilter::Info,
		))
		.build(Root::builder().appender("stdout".to_string()).build(
			log::LevelFilter::Info,
		));
	log4rs::init_config(config.unwrap()).unwrap();


	#[cfg(profiler)]
		cpuprofiler::PROFILER.lock().unwrap().start("./rust-oids.profile").unwrap();

	app::run(&args);

	#[cfg(profiler)]
		cpuprofiler::PROFILER.lock().unwrap().stop().unwrap();
}
