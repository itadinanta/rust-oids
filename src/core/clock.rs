use std::time;

pub type SystemStopwatch = time::SystemTime;

pub trait Stopwatch: Sized {
	fn new() -> Self;

	fn seconds(&self) -> f32;

	fn reset(&mut self) {
		*self = Self::new();
	}

	fn restart(&mut self) -> f32 {
		let elapsed = self.seconds();
		self.reset();
		elapsed
	}
}

pub struct Hourglass<T: Stopwatch> {
	stopwatch: T,
	capacity: f32,
	timeout: f32,
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

impl Stopwatch for SystemStopwatch {
	fn new() -> Self {
		time::SystemTime::now()
	}

	fn seconds(&self) -> f32 {
		match self.elapsed() {
			Ok(dt) => (dt.as_secs() as f32) + (dt.subsec_nanos() as f32) * 1e-9,
			Err(_) => 0.0,
		}
	}
}
