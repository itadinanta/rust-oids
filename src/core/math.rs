use cgmath;
use cgmath::EuclideanVector;
use num;
use std::ops;


pub trait Smooth<S> {
	fn smooth(&mut self, value: S) -> S {
		value
	}
}

pub struct MovingAverage<S: num::Num> {
	ptr: usize,
	count: usize,
	acc: S,
	last: S,
	values: Vec<S>,
}

#[derive(Copy, Clone)]
pub struct Exponential<S, T> {
	tau: T,
	dt: T,
	last: S,
}

impl<S: num::Num + num::NumCast + Copy> MovingAverage<S> {
	pub fn new(window_size: usize) -> Self {
		MovingAverage {
			ptr: 0,
			count: 0,
			last: S::zero(),
			acc: S::zero(),
			values: vec![S::zero(); window_size],
		}
	}
}

impl<S: num::Num + num::NumCast + Copy> Smooth<S> for MovingAverage<S> {
	fn smooth(&mut self, value: S) -> S {
		let len = self.values.len();
		if self.count < len {
			self.count = self.count + 1;
		} else {
			self.acc = self.acc - self.values[self.ptr];
		}
		self.acc = self.acc + value;
		self.values[self.ptr] = value;
		self.ptr = ((self.ptr + 1) % len) as usize;
		self.last = self.acc / num::cast(self.count).unwrap();
		self.last
	}
}

impl<S, T> Exponential<S, T>
	where S: ops::Add<S, Output = S> + ops::Mul<T, Output = S> + Copy,
	      T: cgmath::BaseFloat
{
	pub fn new(value: S, dt: T, tau: T) -> Self {
		Exponential {
			last: value,
			dt: dt,
			tau: tau,
		}
	}

	pub fn reset(&mut self, value: S) {
		self.last = value;
	}

	pub fn dt(&mut self, dt: T) -> &mut Self {
		self.dt = dt;
		self
	}
}

impl<S, T> Smooth<S> for Exponential<S, T>
	where S: ops::Add<S, Output = S> + ops::Mul<T, Output = S> + Copy,
	      T: cgmath::BaseFloat
{
	fn smooth(&mut self, value: S) -> S {
		let one = T::one();
		let alpha = one - T::exp(-self.dt / self.tau);
		self.last = value * alpha + self.last * (one - alpha);
		self.last
	}
}

pub enum Direction {
	Up,
	Down,
	Left,
	Right,
}

pub trait Directional<T: cgmath::BaseFloat> {
	fn push(&mut self, d: Direction);
	fn position(&self) -> cgmath::Vector2<T>;
	fn unit(d: Direction) -> cgmath::Vector2<T> {
		match d {
			Direction::Up => cgmath::Vector2::unit_y(),
			Direction::Down => -cgmath::Vector2::unit_y(),
			Direction::Right => cgmath::Vector2::unit_x(),
			Direction::Left => -cgmath::Vector2::unit_x(),
		}
	}
}

#[derive(Clone)]
pub struct Inertial<T: cgmath::BaseNum + ops::Neg + Copy> {
	impulse: T,
	inertia: T,
	limit: T,
	position: cgmath::Vector2<T>,
	velocity: cgmath::Vector2<T>,
}

impl<T> Default for Inertial<T>
    where T: cgmath::BaseFloat + cgmath::Zero + cgmath::One
{
	fn default() -> Self {
		Inertial {
			impulse: T::one(),
			inertia: T::one(),
			limit: T::one(),
			position: cgmath::Vector::zero(),
			velocity: cgmath::Vector::zero(),
		}
	}
}

impl<T> Directional<T> for Inertial<T>
    where T: cgmath::BaseFloat
{
	fn push(&mut self, d: Direction) {
		let v = Self::unit(d);
		self.velocity = self.velocity + v * self.impulse;
		if self.velocity.length() > self.limit {
			self.velocity.normalize_to(self.limit);
		}
	}
	fn position(&self) -> cgmath::Vector2<T> {
		self.position
	}
}

impl<T> Inertial<T>
    where T: cgmath::BaseFloat
{
	pub fn new(impulse: T, inertia: T, limit: T) -> Self {
		Inertial {
			impulse: impulse,
			inertia: inertia,
			limit: limit,
			..Default::default()
		}
	}

	pub fn reset(&mut self) {
		self.position = cgmath::Vector::zero();
		self.velocity = cgmath::Vector::zero();
	}

	pub fn set(&mut self, position: cgmath::Point2<T>) {
		self.position = cgmath::Point::to_vec(position);
	}

	pub fn stop(&mut self) {
		self.velocity = cgmath::Vector::zero();
	}

	pub fn update(&mut self, dt: T) {
		self.position = self.position + self.velocity * dt;
		self.velocity = self.velocity - self.velocity * T::exp(-dt / self.inertia);
	}
}
