use std::time;
use std::fmt;

pub trait Stopwatch: Sized {
	fn new() -> Self;

	fn seconds(&self) -> f32;

	fn reset(&mut self) {
		*self = Self::new();
	}

	fn tick(&mut self, dt: f32);

	fn restart(&mut self) -> f32 {
		let elapsed = self.seconds();
		self.reset();
		elapsed
	}
}

pub type SystemStopwatch = time::SystemTime;

#[derive(Clone)]
pub struct SimulationStopwatch {
	seconds: f32
}

#[derive(Clone)]
pub struct Hourglass<T: Stopwatch> {
	stopwatch: T,
	capacity: f32,
	timeout: f32,
}

impl<T: Stopwatch> fmt::Debug for Hourglass<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{} ({}, {})", self.left(), self.timeout, self.capacity)
	}
}

impl<T: Stopwatch> fmt::Display for Hourglass<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{} ({}, {})", self.left(), self.timeout, self.capacity)
	}
}

impl<T: Stopwatch> Hourglass<T> {
	pub fn new(seconds: f32) -> Self {
		Hourglass {
			stopwatch: T::new(),
			capacity: seconds,
			timeout: seconds,
		}
	}

	pub fn renew(&mut self) {
		self.timeout = self.capacity;
		self.stopwatch.reset()
	}

	pub fn flip(&mut self) -> f32 {
		let left = self.left();
		self.timeout = self.capacity - left;
		self.stopwatch.reset();
		left
	}

	pub fn seconds(&self) -> f32 {
		self.stopwatch.seconds()
	}

	pub fn left(&self) -> f32 {
		let dt = self.timeout - self.stopwatch.seconds();
		f32::max(0., dt)
	}

	pub fn is_expired(&self) -> bool {
		let dt = self.left();
		let e = dt <= 0.;
		e
	}
}

impl Stopwatch for SimulationStopwatch {
	fn new() -> Self {
		SimulationStopwatch { seconds: 0.0f32 }
	}
	fn seconds(&self) -> f32 {
		self.seconds
	}

	fn tick(&mut self, dt: f32) {
		self.seconds += dt;
	}
}

impl SimulationStopwatch {
	pub fn sync_from<T: Stopwatch>(&mut self, source: &T) {
		self.seconds = source.seconds();
	}
}

impl Stopwatch for SystemStopwatch {
	fn new() -> Self {
		time::SystemTime::now()
	}

	fn tick(&mut self, _: f32) {
	}

	fn seconds(&self) -> f32 {
		match self.elapsed() {
			Ok(dt) => (dt.as_secs() as f32) + (dt.subsec_nanos() as f32) * 1e-9,
			Err(_) => 0.0,
		}
	}
}
