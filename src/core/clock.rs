use std::time;

pub type SystemStopwatch = time::SystemTime;

pub trait Stopwatch: Sized {
	fn new() -> Self;

	fn seconds(&self) -> f32;

	fn reset(&mut self);

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
	fn new(seconds: f32) -> Self {
		Hourglass {
			stopwatch: T::new(),
			capacity: seconds,
			timeout: seconds,
		}
	}

	fn renew(&mut self) {
		self.timeout = self.capacity;
		self.stopwatch.reset()
	}

	fn flip(&mut self) -> f32 {
		let left = self.left();
		self.timeout = self.capacity - left;
		self.stopwatch.reset();
		left
	}

	fn left(&self) -> f32 {
		f32::max(0., self.stopwatch.seconds() - self.timeout)
	}

	fn is_expired(&self) -> bool {
		self.left() <= 0.
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

	fn reset(&mut self) {
		*self = time::SystemTime::now();
	}
}
