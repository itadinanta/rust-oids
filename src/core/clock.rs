use num;
use num::NumCast;
use num::Zero;
use std::fmt;
use std::fmt::Display;
use std::ops::*;
use std::time;

pub type SecondsValue = f64;
pub type SpeedFactor = SecondsValue;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
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

pub fn seconds<T>(value: T) -> Seconds
where T: Into<SecondsValue> {
	Seconds::new(value.into())
}

impl Seconds {
	pub fn new(value: SecondsValue) -> Seconds { Seconds(value) }
	pub fn get(&self) -> SecondsValue { self.0 }
	pub fn times<F>(&self, other: F) -> Seconds
	where F: num::Float {
		seconds(<f64 as NumCast>::from(other).unwrap() * self.0)
	}
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
	fn add(self, other: Seconds) -> Seconds { Seconds(self.0 + other.0) }
}

impl AddAssign for Seconds {
	fn add_assign(&mut self, other: Seconds) { self.0 += other.0; }
}

impl SubAssign for Seconds {
	fn sub_assign(&mut self, other: Seconds) { self.0 -= other.0; }
}

impl<F> Mul<F> for Seconds
where F: num::Float
{
	type Output = F;
	fn mul(self, other: F) -> F { other * <F as NumCast>::from(self.0).unwrap() }
}

impl Div<usize> for Seconds {
	type Output = Seconds;
	fn div(self, other: usize) -> Seconds { Seconds(self.0 / other as SecondsValue) }
}

impl Sub for Seconds {
	type Output = Seconds;
	fn sub(self, other: Seconds) -> Seconds { Seconds(self.0 - other.0) }
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
}

/// SystemTimer
#[derive(Clone)]
pub struct SystemTimer {
	t0: time::SystemTime,
}

impl SystemTimer {
	pub fn new() -> Self {
		SystemTimer {
			t0: time::SystemTime::now(),
		}
	}
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
	seconds: Seconds,
}

impl From<Seconds> for SimulationTimer {
	fn from(seconds: Seconds) -> Self { SimulationTimer { seconds } }
}

impl Timer for SimulationTimer {
	fn seconds(&self) -> Seconds { self.seconds }
}

impl SimulationTimer {
	pub fn new() -> Self {
		SimulationTimer {
			seconds: Seconds::zero(),
		}
	}
	pub fn tick(&mut self, dt: Seconds) { self.seconds += dt }
	//pub fn from<T>(source: T) -> Self where T: Timer { SimulationTimer {
	// seconds: source.seconds() } }
}

/// Stopwatch
pub trait Stopwatch {
	fn reset<T>(&mut self, timer: &T)
	where T: Timer;

	fn elapsed<T>(&self, timer: &T) -> Seconds
	where T: Timer;

	fn restart<T>(&mut self, timer: &T) -> Seconds
	where T: Timer {
		let elapsed = self.elapsed(timer);
		self.reset(timer);
		elapsed
	}
}

#[derive(Clone)]
pub struct TimerStopwatch {
	t0: Seconds,
}

impl TimerStopwatch {
	pub fn new(timer: &Timer) -> Self {
		let t0 = timer.seconds();
		TimerStopwatch { t0 }
	}
}

impl Stopwatch for TimerStopwatch {
	fn reset<T>(&mut self, timer: &T)
	where T: Timer {
		self.t0 = timer.seconds();
	}

	fn elapsed<T>(&self, timer: &T) -> Seconds
	where T: Timer {
		timer.seconds() - self.t0
	}
}

/// Hourglass
#[derive(Clone)]
pub struct Hourglass {
	stopwatch: TimerStopwatch,
	capacity: Seconds,
	timeout: Seconds,
}

impl fmt::Debug for Hourglass {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "({}, {})", self.timeout, self.capacity) }
}

#[allow(unused)]
impl Hourglass {
	pub fn new(seconds: Seconds, timer: &Timer) -> Self {
		Hourglass {
			stopwatch: TimerStopwatch::new(timer),
			capacity: seconds,
			timeout: seconds,
		}
	}

	pub fn renew<T>(&mut self, timer: &T)
	where T: Timer {
		self.timeout = self.capacity;
		self.stopwatch.reset(timer)
	}

	pub fn flip<T>(&mut self, timer: &T) -> Seconds
	where T: Timer {
		let left = self.left(timer);
		self.timeout = self.capacity - left;
		self.stopwatch.reset(timer);
		left
	}

	pub fn delay(&mut self, delay_seconds: Seconds) { self.timeout = self.timeout + delay_seconds; }

	#[allow(unused)]
	pub fn elapsed<T>(&self, timer: &T) -> Seconds
	where T: Timer {
		self.stopwatch.elapsed(timer)
	}

	pub fn left<T>(&self, timer: &T) -> Seconds
	where T: Timer {
		let dt = self.timeout - self.stopwatch.elapsed(timer);
		Seconds(SecondsValue::max(0., dt.into()))
	}

	pub fn is_expired<T>(&self, timer: &T) -> bool
	where T: Timer {
		self.left(timer).get() <= Seconds::zero().0
	}

	pub fn flip_if_expired<T>(&mut self, timer: &T) -> bool
	where T: Timer {
		let expired = self.is_expired(timer);
		if expired {
			self.flip(timer);
		};
		expired
	}
}
