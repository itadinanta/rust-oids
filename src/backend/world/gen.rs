use num;
use std::f32::consts;
use rand;
use rand::Rng;
use backend::obj::*;

#[allow(dead_code)]
pub trait Generator {
	fn next_float<T>(&mut self, min: T, max: T) -> T where T: rand::Rand + num::Float;

	fn next_integer<T>(&mut self, min: T, max: T) -> T where T: rand::Rand + num::Integer + Copy;

	fn ball(&mut self) -> Shape {
		let radius: f32 = self.next_float(1.0, 2.0);
		Shape::new_ball(radius)
	}

	fn quad(&mut self) -> Shape {
		let radius: f32 = self.next_float(1.0, 2.0);
		let ratio: f32 = self.next_float(1.0, 2.0);
		Shape::new_box(radius, ratio)
	}

	fn vbar(&mut self) -> Shape {
		let radius: f32 = self.next_float(1.0, 2.0);
		let ratio: f32 = self.next_float(0.1, 0.2);
		Shape::new_box(radius, ratio)
	}

	fn triangle(&mut self) -> Shape {
		let radius = self.next_float(0.5, 1.0);
		let alpha1 = self.next_float(consts::PI * 0.5, consts::PI * 0.9);
		let alpha2 = consts::PI * 1.5 - self.next_float(0., consts::PI);
		Shape::new_triangle(radius, alpha1, alpha2)
	}

	fn iso_triangle(&mut self) -> Shape {
		let radius = self.next_float(0.5, 1.0);
		let alpha1 = self.next_float(consts::PI * 0.5, consts::PI * 0.9);
		let alpha2 = consts::PI * 2. - alpha1;
		Shape::new_triangle(radius, alpha1, alpha2)
	}

	fn eq_triangle(&mut self) -> Shape {
		let radius = self.next_float(0.5, 1.0);
		let alpha1 = consts::PI * 2. / 3.;
		let alpha2 = consts::PI * 2. - alpha1;
		Shape::new_triangle(radius, alpha1, alpha2)
	}

	fn star(&mut self) -> Shape {
		let radius: f32 = self.next_float(1.0, 2.0);
		let n = self.next_integer(3, 8);
		let ratio1 = self.next_float(0.5, 1.0);
		let ratio2 = self.next_float(0.7, 0.9) * (1. / ratio1);
		Shape::new_star(n, radius, ratio1, ratio2)
	}

	fn poly(&mut self, upside_down: bool) -> Shape {
		let n = self.next_integer(3, 8);
		self.npoly(n, upside_down)
	}

	fn npoly(&mut self, n: AttachmentIndex, upside_down: bool) -> Shape {
		let radius: f32 = self.next_float(1.0, 2.0);
		let ratio1 = f32::cos(consts::PI / n as f32);
		let ratio2 = 1. / ratio1;
		if upside_down {
			Shape::new_star(n, radius * ratio1, ratio2, ratio1)
		} else {
			Shape::new_star(n, radius, ratio1, ratio2)
		}
	}
}

pub struct Randomizer {
	rng: rand::ThreadRng,
}

impl Randomizer {
	pub fn new() -> Self {
		Randomizer { rng: rand::thread_rng() }
	}
}

impl Generator for Randomizer {
	fn next_float<T>(&mut self, min: T, max: T) -> T
		where T: rand::Rand + num::Float {
		self.rng.gen::<T>() * (max - min) + min
	}

	fn next_integer<T>(&mut self, min: T, max: T) -> T
		where T: rand::Rand + num::Integer + Copy {
		self.rng.gen::<T>() % (max - min + T::one()) + min
	}
}
