use std::fmt;
use std::f32::consts;
use std::u16;
use num;
use std::cmp;
use rand;
use rand::Rng;
use backend::obj::*;
use serialize::base64::{self, ToBase64};

pub type Dna = Box<[u8]>;

const MAX_POLY_SIDES: u8 = 8; // in conformity with box2d?

#[allow(dead_code)]
pub trait Generator {
	fn next_float<T>(&mut self, min: T, max: T) -> T where T: rand::Rand + num::Float;

	fn next_integer<T>(&mut self, min: T, max: T) -> T
		where T: rand::Rand + num::Integer + num::ToPrimitive + num::FromPrimitive + Copy;

	fn next_bool(&mut self) -> bool {
		self.next_integer::<u8>(0, 1) == 1
	}

	fn ball(&mut self) -> Shape {
		let radius: f32 = self.next_float(0.5, 0.75);
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
		let alpha1 = self.next_float(consts::PI * 0.5, consts::PI * 0.8);
		let alpha2 = self.next_float(consts::PI * 1.2, consts::PI * 1.5);
		Shape::new_triangle(radius, alpha1, alpha2)
	}

	fn iso_triangle(&mut self) -> Shape {
		let radius = self.next_float(0.5, 1.0);
		let alpha1 = self.next_float(consts::PI * 0.5, consts::PI * 0.8);
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
		// if pie slices are too small physics freaks out
		let n = self.next_integer(3,
		                          if radius > 1.5 { MAX_POLY_SIDES } else { MAX_POLY_SIDES - 2 });
		let ratio1 = self.next_float(0.5, 1.0);
		let ratio2 = self.next_float(0.7, 0.9) * (1. / ratio1);
		Shape::new_star(n, radius, ratio1, ratio2)
	}

	fn poly(&mut self, upside_down: bool) -> Shape {
		let n = self.next_integer(3, MAX_POLY_SIDES);
		self.npoly(n, upside_down)
	}

	fn any_poly(&mut self) -> Shape {
		let n = self.next_integer(3, MAX_POLY_SIDES);
		let upside_down = self.next_bool();
		self.npoly(n, upside_down)
	}

	fn npoly(&mut self, n: AttachmentIndex, upside_down: bool) -> Shape {
		let radius: f32 = self.next_float(1.0, 2.0);
		let ratio1 = f32::cos(consts::PI / n as f32);
		let corrected_radius = if upside_down { radius * ratio1 } else { radius };

		if n <= MAX_POLY_SIDES {
			Shape::new_poly(if upside_down { -1 } else { 1 } * n as i8, corrected_radius)
		} else {
			let ratio2 = 1. / ratio1;
			if upside_down {
				Shape::new_star(n, corrected_radius, ratio2, ratio1)
			} else {
				Shape::new_star(n, corrected_radius, ratio1, ratio2)
			}
		}
	}
}

#[allow(dead_code)]
pub struct Randomizer {
	rng: rand::ThreadRng,
}

#[allow(dead_code)]
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
	dna: Box<[u8]>,
	ptr: usize,
}

impl Genome {
	pub fn new(dna: &[u8]) -> Self {
		Genome {
			ptr: 0,
			dna: dna.to_owned().into_boxed_slice(),
		}
	}

	fn next_byte(&mut self) -> u8 {
		let next = self.dna[self.ptr];
		self.ptr = (self.ptr + 1) % self.dna.len();
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

	pub fn crossover<R: rand::Rng>(&self, rng: &mut R, other: &Dna) -> Self {
		let len = cmp::min(self.dna.len(), other.len());
		let p: usize = rng.gen::<usize>() % (len * 8);
		let byte = p / 8;
		let bit = p % 8;
		let flip_mask = if rng.gen::<bool>() { 0xffu8 } else { 0x0u8 };
		let mut new_genes = self.dna.to_vec();
		for i in 0..len {
			let a = new_genes[i];
			let b = other[i];
			let mask = if i < byte || (bit == 0 && i == byte) {
				0x00u8
			} else if i > byte {
				0xffu8
			} else {
				(0xffu8 >> (8 - bit)) as u8
			} ^ flip_mask;
			new_genes[i] = (mask & a) | (!mask & b);
		}

		println!("crossover at {}: {} * {} -> {}",
		         p,
		         self.dna.to_base64(base64::STANDARD),
		         other.to_base64(base64::STANDARD),
		         new_genes.to_base64(base64::STANDARD));

		Genome {
			ptr: 0,
			dna: new_genes.into_boxed_slice(),
		}
	}

	pub fn mutate<R: rand::Rng>(&self, rng: &mut R) -> Self {
		let p: usize = rng.gen::<usize>() % (self.dna.len() * 8);
		let mut new_genes = self.dna.to_vec();
		let byte = p / 8;
		let bit = p % 8;
		new_genes[byte] ^= 1 << bit;
		Genome {
			ptr: 0,
			dna: new_genes.into_boxed_slice(),
		}
	}

	pub fn dna(&self) -> &Box<[u8]> {
		&self.dna
	}
}

impl fmt::Display for Genome {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.dna.to_base64(base64::STANDARD))
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
