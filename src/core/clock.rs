use std::time;
use std::fmt;
use std::fmt::Display;
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::*;
use num::Zero;

pub type SecondsValue = f64;
pub type SpeedFactor = SecondsValue;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seconds(SecondsValue);

const ZERO_SECONDS: Seconds = Seconds(0.0);

impl Display for Seconds {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		if self.0 > 0.5 as SecondsValue {
			write!(f, "{:.3}s", self.0)
		} else {
			write!(f, "{:.1}ms", self.0 * 1000.0)
		}
	}
}

pub fn seconds(value: SecondsValue) -> Seconds {
	Seconds::new(value)
}

impl Seconds {
	pub fn new(value: SecondsValue) -> Seconds { Seconds(value) }
	pub fn get(&self) -> SecondsValue { self.0 }
}

impl Zero for Seconds {
	#[inline]
	fn zero() -> Seconds { ZERO_SECONDS }
	fn is_zero(&self) -> bool { *self == ZERO_SECONDS }
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

impl Mul<SecondsValue> for Seconds {
	type Output = Seconds;
	fn mul(self, other: SecondsValue) -> Seconds {
		Seconds(self.0 * other)
	}
}

impl Div<usize> for Seconds {
	type Output = Seconds;
	fn div(self, other: usize) -> Seconds {
		Seconds(self.0 / other as SecondsValue)
	}
}

impl Sub for Seconds {
	type Output = Seconds;
	fn sub(self, other: Seconds) -> Seconds {
		Seconds(self.0 - other.0)
	}
}

impl Into<SecondsValue> for Seconds {
	fn into(self) -> SecondsValue { self.0 }
}

impl Into<f32> for Seconds {
	fn into(self) -> f32 { self.0 as f32 }
}

/// Timer
pub trait Timer {
	fn seconds(&self) -> Seconds;
	fn shared(self) -> Rc<RefCell<Self>> where Self: Sized {
		Rc::new(RefCell::new(self))
	}
}

/// SystemTimer
#[derive(Clone)]
pub struct SystemTimer {
	t0: time::SystemTime,
}

impl SystemTimer {
	pub fn new() -> Self { SystemTimer { t0: time::SystemTime::now() } }
}

impl Timer for SystemTimer {
	fn seconds(&self) -> Seconds {
		match self.t0.elapsed() {
			Ok(dt) => Seconds((dt.as_secs() as SecondsValue) + (dt.subsec_nanos() as SecondsValue) * 1e-9),
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
		self.seconds += dt
	}
}

/// Stopwatch
pub trait Stopwatch {
	fn reset(&mut self);

	fn elapsed(&self) -> Seconds;

	fn restart(&mut self) -> Seconds {
		let elapsed = self.elapsed();
		self.reset();
		elapsed
	}
}

pub type SharedTimer<T> = Rc<RefCell<T>>;

/// TimerStopwatch
#[derive(Clone)]
pub struct TimerStopwatch<T> where T: Timer {
	timer: SharedTimer<T>,
	t0: Seconds,
}

impl<T> Stopwatch for TimerStopwatch<T> where T: Timer {
	fn elapsed(&self) -> Seconds {
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

	pub fn elapsed(&self) -> Seconds {
		self.stopwatch.elapsed()
	}

	pub fn left(&self) -> Seconds {
		let dt = self.timeout - self.stopwatch.elapsed();
		Seconds(SecondsValue::max(0., dt.into()))
	}

	pub fn is_expired(&self) -> bool {
		self.left().get() <= Seconds::zero().0
	}

	pub fn flip_if_expired(&mut self) -> bool {
		let expired = self.is_expired();
		if expired { self.flip(); };
		expired
	}
}
