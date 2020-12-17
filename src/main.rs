#![allow(unknown_lints)]
#![warn(clippy::all)]

mod app;
mod backend;
mod core;
mod frontend;

#[macro_use]
extern crate log;
extern crate chrono;
extern crate csv;
extern crate log4rs;

#[macro_use]
extern crate bitflags;
extern crate bit_set;
extern crate cgmath;

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

extern crate wrapped2d;

#[macro_use]
extern crate gfx;
extern crate gfx_device_gl;
extern crate gl;
extern crate glutin;
extern crate image;
extern crate winit;

extern crate dasp;
extern crate dasp_sample;
extern crate dasp_signal;
extern crate dasp_slice;
extern crate pitch_calc;
extern crate portaudio;

extern crate itertools;
extern crate num;
extern crate num_traits;
extern crate rand;

#[cfg(feature = "profiler")]
extern crate cpuprofiler;

#[macro_use]
extern crate enum_primitive;
extern crate conrod;

extern crate ctrlc;
extern crate getopts;

extern crate gilrs;

extern crate dirs;
extern crate rayon;
#[cfg(target_os = "linux")]
extern crate thread_priority;

extern crate rustc_serialize as serialize;

fn main() {
	use log4rs::append::console::*;
	use log4rs::config::*;
	use std::env;
	let args = env::args_os().collect::<Vec<_>>();

	let config = Config::builder()
		.appender(Appender::builder().build("stdout".to_string(), Box::new(ConsoleAppender::builder().build())))
		.logger(Logger::builder().build("gfx_device_gl".to_string(), log::LevelFilter::Error))
		.logger(Logger::builder().build("rust_oids".to_string(), log::LevelFilter::Info))
		.build(Root::builder().appender("stdout".to_string()).build(log::LevelFilter::Info));
	log4rs::init_config(config.unwrap()).unwrap();

	#[cfg(feature = "profiler")]
	{
		cpuprofiler::PROFILER.lock().unwrap().start("./rust-oids.profile").unwrap();
	}

	app::run(&args);

	#[cfg(feature = "profiler")]
	{
		cpuprofiler::PROFILER.lock().unwrap().stop().unwrap();
	}
}
