use cgmath;
use cgmath::InnerSpace;
use num::Zero;
use std::ops::*;
use num;
use num::NumCast;
use num_traits::FloatConst;

pub trait Smooth<S> {
	fn smooth(&mut self, value: S) -> S {
		value
	}
}

#[allow(unused)]
pub fn normalize_rad<S>(angle: S) -> S
	where S: num::Float + FloatConst {
	let pi: S = S::PI();
	(angle
		+ <S as NumCast>::from(3.).unwrap() * pi)
		% (<S as NumCast>::from(2.).unwrap() * pi)
		- pi
}

pub struct MovingAverage<S> {
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

impl<S: Zero + Copy> MovingAverage<S> {
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

impl<S> Smooth<S> for MovingAverage<S>
	where
		S: Zero + Sub + Copy + AddAssign + SubAssign + Div<usize, Output=S>,
{
	fn smooth(&mut self, value: S) -> S {
		let len = self.values.len();
		if self.count < len {
			self.count = self.count + 1;
		} else {
			self.acc -= self.values[self.ptr];
		}
		self.acc += value;
		self.values[self.ptr] = value;
		self.ptr = ((self.ptr + 1) % len) as usize;
		self.last = self.acc / self.count;
		self.last
	}
}

impl<S, T> Exponential<S, T>
	where
		S: Add<S, Output=S> + Mul<T, Output=S> + Copy,
		T: cgmath::BaseFloat,
{
	pub fn new(value: S, dt: T, tau: T) -> Self {
		Exponential {
			last: value,
			dt,
			tau,
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
	where
		S: Add<S, Output=S> + Mul<T, Output=S> + Copy,
		T: cgmath::BaseFloat,
{
	fn smooth(&mut self, value: S) -> S {
		let alpha1 = T::exp(-self.dt / self.tau);
		self.last = value * (T::one() - alpha1) + self.last * alpha1;
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
	fn push(&mut self, d: Direction, weight: T);
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

pub trait Relative<T: cgmath::BaseFloat> {
	fn zero(&mut self);
	fn set_relative(&mut self, p: cgmath::Vector2<T>);
}

#[derive(Clone)]
pub struct Inertial<T: cgmath::BaseNum + Neg + Copy> {
	impulse: T,
	inertia: T,
	limit: T,
	target: Option<cgmath::Vector2<T>>,
	zero: cgmath::Vector2<T>,
	position: cgmath::Vector2<T>,
	velocity: cgmath::Vector2<T>,
}

impl<T> Default for Inertial<T>
	where
		T: cgmath::BaseFloat + cgmath::Zero + cgmath::One,
{
	fn default() -> Self {
		Inertial {
			impulse: T::one(),
			inertia: T::one(),
			limit: T::one(),
			target: None,
			zero: cgmath::Zero::zero(),
			position: cgmath::Zero::zero(),
			velocity: cgmath::Zero::zero(),
		}
	}
}

impl<T> Directional<T> for Inertial<T>
	where
		T: cgmath::BaseFloat,
{
	fn push(&mut self, d: Direction, weight: T) {
		let v = Self::unit(d) * weight;
		self.velocity = self.velocity + v * self.impulse;
		if self.velocity.magnitude() > self.limit {
			self.velocity.normalize_to(self.limit);
		}
	}
	fn position(&self) -> cgmath::Vector2<T> {
		self.position
	}
}

impl<T> Relative<T> for Inertial<T>
	where
		T: cgmath::BaseFloat,
{
	fn zero(&mut self) {
		self.zero = self.position;
	}
	fn set_relative(&mut self, p: cgmath::Vector2<T>) {
		let zero = self.zero;
		self.set(zero + p);
	}
}

#[allow(dead_code)]
impl<T> Inertial<T>
	where
		T: cgmath::BaseFloat,
{
	pub fn new(impulse: T, inertia: T, limit: T) -> Self {
		Inertial {
			impulse,
			inertia,
			limit,
			..Default::default()
		}
	}

	pub fn follow(&mut self, target: Option<cgmath::Vector2<T>>) {
		self.target = target;
	}

	pub fn reset(&mut self) {
		self.position = cgmath::Zero::zero();
		self.velocity = cgmath::Zero::zero();
	}

	pub fn set(&mut self, position: cgmath::Vector2<T>) {
		self.position = position;
	}

	pub fn velocity(&mut self, velocity: cgmath::Vector2<T>) {
		self.velocity = velocity;
	}

	pub fn stop(&mut self) {
		self.velocity = cgmath::Zero::zero();
	}

	pub fn update<D: Into<T>>(&mut self, dt: D) {
		let dt: T = dt.into();
		if let Some(destination) = self.target {
			self.position += (destination - self.position) * self.inertia * dt;
		} else {
			self.position = self.position + self.velocity * dt;
			self.velocity = self.velocity * T::exp(-dt / self.inertia);
		}
	}
}
