use num;
use std::f32::consts;
use std::u16;
use rand;
use rand::Rng;
use backend::obj::*;

#[allow(dead_code)]
pub trait Generator {
	fn next_float<T>(&mut self, min: T, max: T) -> T where T: rand::Rand + num::Float;

	fn next_integer<T>(&mut self, min: T, max: T) -> T
		where T: rand::Rand + num::Integer + num::ToPrimitive + num::FromPrimitive + Copy;

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

pub struct Genome {
	bits: Box<[u8]>,
	ptr: usize,
}

impl Genome {
	pub fn new(bits: &[u8]) -> Self {
		Genome {
			ptr: 0,
			bits: bits.to_owned().into_boxed_slice(),
		}
	}

	fn next_byte(&mut self) -> u8 {
		let next = self.bits[self.ptr];
		self.ptr = (self.ptr + 1) % self.bits.len();
		next
	}

	fn next_bytes(&mut self, n: u8) -> i64 {
		let bytes = (0..n).fold(0, |a, _| a << 8 | self.next_byte() as i64);
		bytes
	}

	fn next_i32(&mut self, min: i32, max: i32) -> i32 {
		let diff = max as i64 - min as i64 + 1i64;
		if diff <= 0 {
			min
		} else {
			for i in 1..4 {
				if diff < (1 << (i * 8)) {
					return (self.next_bytes(i) % diff + min as i64) as i32;
				}
			}
			min
		}
	}

	pub fn mutate<R: rand::Rng>(&self, rng: &mut R) -> Self {
		let p: usize = rng.gen::<usize>() % (self.bits.len() * 8);
		let mut new_genes = self.bits.to_vec();
		let byte = p / 8;
		let bit = p % 8;
		new_genes[byte] ^= 1 << bit;
		Genome {
			ptr: 0,
			bits: new_genes.into_boxed_slice(),
		}
	}

	pub fn dna(&self) -> &Box<[u8]> {
		&self.bits
	}
}

impl Generator for Genome {
	fn next_float<T>(&mut self, min: T, max: T) -> T
		where T: rand::Rand + num::Float {
		let u0 = self.next_bytes(2) as u16;
		let n: T = T::from(u0).unwrap() / T::from(u16::MAX).unwrap();
		n * (max - min) + min
	}

	fn next_integer<T>(&mut self, min: T, max: T) -> T
		where T: rand::Rand + num::Integer + num::ToPrimitive + num::FromPrimitive + Copy {
		num::NumCast::from(min)
			.and_then(|a| num::NumCast::from(max).map(|b| self.next_i32(a, b)))
			.and_then(|value| num::FromPrimitive::from_i32(value))
			.unwrap_or(min)
	}
}
