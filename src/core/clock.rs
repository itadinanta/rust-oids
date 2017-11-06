use std::time;
use std::fmt;
use std::fmt::Display;
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::*;
use num::Zero;

#[derive(Clone, Copy, Debug)]
pub struct Seconds(f32);

impl Display for Seconds {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}s", self.0)
	}
}

impl Seconds {
	pub fn new(value: f32) -> Seconds { Seconds(value) }
	pub fn to_f32(&self) -> f32 { self.0 }
}

impl Zero for Seconds {
	fn zero() -> Seconds { Seconds(0.0f32) }
	fn is_zero(&self) -> bool { self.0 == 0.0f32 }
}

impl Default for Seconds {
	fn default() -> Seconds { Seconds::zero() }
}

impl Add for Seconds {
	type Output = Seconds;
	fn add(self, other: Seconds) -> Seconds {
		Seconds(self.0 + other.0)
	}
}

impl AddAssign for Seconds {
	fn add_assign(&mut self, other: Seconds) {
		self.0 += other.0;
	}
}

impl SubAssign for Seconds {
	fn sub_assign(&mut self, other: Seconds) {
		self.0 -= other.0;
	}
}


impl Mul<f32> for Seconds {
	type Output = Seconds;
	fn mul(self, other: f32) -> Seconds {
		Seconds(self.0 * other)
	}
}

impl Div<usize> for Seconds {
	type Output = Seconds;
	fn div(self, other: usize) -> Seconds {
		Seconds(self.0 / other as f32)
	}
}

impl Sub for Seconds {
	type Output = Seconds;
	fn sub(self, other: Seconds) -> Seconds {
		Seconds(self.0 - other.0)
	}
}

impl Into<f32> for Seconds {
	fn into(self) -> f32 { self.0 }
}

/// Timer
pub trait Timer {
	fn seconds(&self) -> Seconds;
}

/// SystemTimer
#[derive(Clone)]
pub struct SystemTimer {}

impl SystemTimer {
	pub fn new() -> Self { SystemTimer {} }
}

impl Timer for SystemTimer {
	fn seconds(&self) -> Seconds {
		let now = time::SystemTime::now();
		match now.elapsed() {
			Ok(dt) => Seconds((dt.as_secs() as f32) + (dt.subsec_nanos() as f32) * 1e-9),
			Err(_) => Seconds::zero(),
		}
	}
}

/// SimulationTimer
#[derive(Clone)]
pub struct SimulationTimer {
	seconds: Seconds
}

impl SimulationTimer {
	pub fn new() -> Self { SimulationTimer { seconds: Seconds::zero() } }
}

impl From<Seconds> for SimulationTimer {
	fn from(seconds: Seconds) -> Self { SimulationTimer { seconds } }
}

impl Timer for SimulationTimer {
	fn seconds(&self) -> Seconds {
		self.seconds
	}
}

impl SimulationTimer {
	pub fn tick(&mut self, dt: Seconds) {
		self.seconds = self.seconds + dt
	}
}

/// Stopwatch
pub trait Stopwatch {
	fn reset(&mut self);

	fn seconds(&self) -> Seconds;

	fn restart(&mut self) -> Seconds {
		let elapsed = self.seconds();
		self.reset();
		elapsed
	}
}

pub type SharedTimer<T> = Rc<RefCell<T>>;

/// TimerStopwatch
#[derive(Clone)]
pub struct TimerStopwatch<T> where T: Timer {
	timer: SharedTimer<T>,
	t0: Seconds
}

impl<T> Stopwatch for TimerStopwatch<T> where T: Timer {
	fn seconds(&self) -> Seconds {
		self.timer.borrow().seconds() - self.t0
	}

	fn reset(&mut self) {
		self.t0 = self.timer.borrow().seconds();
	}
}

impl<T> TimerStopwatch<T> where T: Timer {
	pub fn new(timer: SharedTimer<T>) -> Self {
		let t0 = timer.borrow().seconds();
		TimerStopwatch { timer, t0 }
	}
}

/// Hourglass
#[derive(Clone)]
pub struct Hourglass<T: Timer> {
	stopwatch: TimerStopwatch<T>,
	capacity: Seconds,
	timeout: Seconds,
}

impl<T> fmt::Debug for Hourglass<T> where T: Timer {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{} ({}, {})", self.left(), self.timeout, self.capacity)
	}
}

impl<T> fmt::Display for Hourglass<T> where T: Timer {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{} ({}, {})", self.left(), self.timeout, self.capacity)
	}
}

impl<T> Hourglass<T> where T: Timer {
	pub fn new(timer: SharedTimer<T>, seconds: Seconds) -> Self {
		Hourglass {
			stopwatch: TimerStopwatch::new(timer),
			capacity: seconds,
			timeout: seconds,
		}
	}

	pub fn renew(&mut self) {
		self.timeout = self.capacity;
		self.stopwatch.reset()
	}

	pub fn flip(&mut self) -> Seconds {
		let left = self.left();
		self.timeout = self.capacity - left;
		self.stopwatch.reset();
		left
	}

	pub fn seconds(&self) -> Seconds {
		self.stopwatch.seconds()
	}

	pub fn left(&self) -> Seconds {
		let dt = self.timeout - self.stopwatch.seconds();
		Seconds(f32::max(0., dt.into()))
	}

	pub fn is_expired(&self) -> bool {
		let dt = self.left();
		let e: bool = <Seconds as Into<f32>>::into(dt) <= 0.0f32;
		e
	}
}
