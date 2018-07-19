use cgmath;
use cgmath::InnerSpace;
use num::Zero;
use std::ops::*;
use std::marker::PhantomData;
use num;
use num::NumCast;
use num_traits::FloatConst;

pub trait Smooth<S> {
	fn smooth(&mut self, value: S) -> S {
		value
	}
}

pub trait IntervalSmooth<S, T>
	where S: Add<S, Output=S> + Mul<T, Output=S> {
	fn smooth(&mut self, value: S, dt: T) -> S { value * dt }
	fn reset(&mut self, _value: S) {}
	fn last(&self) -> S;
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
		S: Zero + Sub + Copy + AddAssign + SubAssign + Div<usize, Output=S>
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

pub trait Mix<V> where V: num::Float {
	fn mix(self, a: V, b: V) -> V;
}

impl<T, V> Mix<V> for T where
	T: num::Float,
	V: num::Float + Mul<T, Output=V> {
	fn mix(self, a: V, b: V) -> V {
		let alpha = T::min(T::one(), T::max(T::zero(), self));
		a * alpha + b * (T::one() - alpha)
	}
}

impl<S, T> Exponential<S, T>
	where
		S: Add<S, Output=S> + Mul<T, Output=S> + Copy,
		T: cgmath::BaseFloat,
{
	pub fn new(value: S, tau: T) -> Self {
		Exponential {
			last: value,
			tau,
		}
	}
}

impl<S, T> IntervalSmooth<S, T> for Exponential<S, T>
	where
		S: Add<S, Output=S> + Mul<T, Output=S> + Copy,
		T: cgmath::BaseFloat,
{
	fn smooth(&mut self, value: S, dt: T) -> S {
		let alpha1 = T::exp(-dt / self.tau);
		self.last = value * (T::one() - alpha1) + self.last * alpha1;
		self.last
	}

	fn reset(&mut self, value: S) {
		self.last = value;
	}

	fn last(&self) -> S {
		self.last
	}
}

#[derive(Clone)]
pub struct LPF<S, T, M> where
	S: Add<S, Output=S> + Mul<T, Output=S> + Copy,
	T: cgmath::BaseFloat,
	M: IntervalSmooth<S, T> {
	target: S,
	smooth: M,
	_interval: PhantomData<T>,
}

impl<S, T, M> LPF<S, T, M> where
	S: Add<S, Output=S> + Mul<T, Output=S> + Copy,
	T: cgmath::BaseFloat,
	M: IntervalSmooth<S, T> {
	pub fn new(initial_target: S, smooth: M) -> Self {
		LPF {
			target: initial_target,
			smooth,
			_interval: PhantomData,
		}
	}

	pub fn input(&mut self, target: S) { self.target = target; }
	pub fn target(&self) -> S { self.target }
	pub fn reset(&mut self) { self.smooth.reset(self.target) }
	pub fn output(&mut self, dt: T) -> S { self.smooth.smooth(self.target, dt) }
	pub fn last_output(&self) -> S { self.smooth.last() }
}

pub type ExponentialFilter<T> = LPF<T, T, Exponential<T, T>>;

pub fn exponential_filter<T>(initial_input: T, initial_output: T, decay: T) -> ExponentialFilter<T>
	where T: cgmath::BaseFloat {
	LPF::new(initial_input, Exponential::new(initial_output, decay))
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
