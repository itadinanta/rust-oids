use backend::obj::*;
use csv;
use num;
use rand;
use rand::Rng;
use serialize::base64::{self, FromBase64, ToBase64};
use std::cmp;
use std::f32::consts;
use std::fmt;
use std::slice::Iter;

pub type Dna = Box<[u8]>;

const MAX_POLY_SIDES: u8 = 8; // in conformity with box2d?

fn bit_count(p: usize) -> usize { p << 3 }

fn split_bit(p: usize) -> (usize, u8) { (p >> 3, (p & 0x7) as u8) }

pub struct GenePool {
	gene_pool: Box<[Dna]>,
	round_robin: usize,
}

impl GenePool {
	pub fn gene_pool_iter(&self) -> Iter<Dna> { self.gene_pool.iter() }
	pub fn gene_pool_index(&self) -> usize { self.round_robin }

	pub fn populate_from_base64(&mut self, base64: &[String], round_robin: usize) {
		self.gene_pool =
			base64.iter().map(|s| s.from_base64().unwrap().into_boxed_slice()).collect::<Vec<_>>().into_boxed_slice();
		self.round_robin = round_robin;
	}

	pub fn parse_from_base64(base64: &[&str]) -> Self {
		GenePool {
			gene_pool: base64
				.iter()
				.map(|s| s.from_base64().unwrap().into_boxed_slice())
				.collect::<Vec<_>>()
				.into_boxed_slice(),
			round_robin: 0,
		}
	}

	pub fn parse_from_resource(data: &[u8]) -> Self {
		let mut gene_pool = Vec::new();
		let mut csv = csv::Reader::from_bytes(data).has_headers(false);
		for row in csv.records() {
			let fields = row.unwrap();
			gene_pool.push(fields[0].from_base64().unwrap().into_boxed_slice());
		}
		GenePool { gene_pool: gene_pool.to_vec().into_boxed_slice(), round_robin: 0 }
	}

	pub fn len(&self) -> usize { self.gene_pool.len() }

	#[allow(dead_code)]
	pub fn new(gene_pool: &[Dna]) -> Self {
		GenePool { gene_pool: gene_pool.to_vec().into_boxed_slice(), round_robin: 0 }
	}

	pub fn randomize(&mut self) {
		let mut rnd = Randomizer::new();
		self.gene_pool[self.round_robin] = rnd.seed().dna_cloned();
	}

	pub fn next(&mut self) -> Genome {
		let gen = Genome::copy_from(&self.gene_pool[self.round_robin].clone());
		let mutated = gen.mutate(&mut rand::thread_rng());
		self.gene_pool[self.round_robin] = mutated.dna_cloned();
		self.round_robin = (self.round_robin + 1) % self.gene_pool.len();
		gen
	}
}

#[allow(dead_code)]
pub trait Generator {
	fn next_float<T>(&mut self, min: T, max: T) -> T
	where T: rand::Rand + num::Float;

	fn next_integer<T>(&mut self, min: T, max: T) -> T
	where T: rand::Rand + num::Integer + num::ToPrimitive + num::FromPrimitive + Copy;

	fn next_bool(&mut self) -> bool { self.next_integer::<u8>(0, 1) == 1 }

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
		let n = self.next_integer(3, if radius > 1.5 { MAX_POLY_SIDES } else { MAX_POLY_SIDES - 2 });
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
		let ratio1 = f32::cos(consts::PI / f32::from(n));
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
pub struct Randomizer<R>
where R: rand::Rng {
	rng: R,
}

#[allow(dead_code)]
impl Randomizer<rand::ThreadRng> {
	pub fn new() -> Randomizer<rand::ThreadRng> { Randomizer { rng: rand::thread_rng() } }
}

impl Generator for Randomizer<rand::ThreadRng> {
	fn next_float<T>(&mut self, min: T, max: T) -> T
	where T: rand::Rand + num::Float {
		self.rng.gen::<T>() * (max - min) + min
	}

	fn next_integer<T>(&mut self, min: T, max: T) -> T
	where T: rand::Rand + num::Integer + Copy {
		self.rng.gen::<T>() % (max - min + T::one()) + min
	}
}

trait Seeder {
	fn seed(&mut self) -> Genome;
}

impl<R> Seeder for Randomizer<R>
where R: rand::Rng
{
	fn seed(&mut self) -> Genome {
		let mut dna = vec![0u8; 72];
		self.rng.fill_bytes(dna.as_mut_slice());
		Genome::new(dna)
	}
}

#[derive(Clone)]
pub struct Genome {
	dna: Box<[u8]>,
	ptr: usize,
	bit_count: usize,
}

impl Genome {
	pub fn copy_from(dna: &[u8]) -> Self {
		Genome { ptr: 0, bit_count: bit_count(dna.len()), dna: dna.to_owned().into_boxed_slice() }
	}

	pub fn new(dna: Vec<u8>) -> Self { Genome { ptr: 0, bit_count: bit_count(dna.len()), dna: dna.into_boxed_slice() } }

	#[inline]
	fn next_bit(&mut self) -> u8 {
		let (byte, bit) = split_bit(self.ptr);
		let next = (self.dna[byte] & (1 << bit)) >> bit;
		self.ptr = (self.ptr + 1) % self.bit_count;
		next
	}

	#[inline]
	fn next_bits(&mut self, n: u8) -> i64 {
		//use std::iter;
		//iter::repeat_with(|| i64::from(self.next_bit())).take(usize::from(n)).fold(0,
		// |a, bit| a << 1 | bit)
		(0..n).fold(0, |a, _| a << 1 | i64::from(self.next_bit()))
	}

	#[inline]
	fn count_bits(d: u64) -> u8 { (64 - d.leading_zeros()) as u8 }

	fn next_i32(&mut self, min: i32, max: i32) -> i32 {
		let diff = i64::from(max) - i64::from(min) + 1i64;
		if diff <= 0 {
			min
		} else {
			(self.next_bits(Self::count_bits(diff as u64)) % diff + i64::from(min)) as i32
		}
	}

	pub fn crossover<R: rand::Rng>(&self, rng: &mut R, other: &Dna) -> Self {
		let len = cmp::min(self.bit_count, bit_count(other.len()));
		let (byte, bit) = split_bit(rng.gen::<usize>() % len);
		let flip_mask = if rng.gen::<bool>() { 0xffu8 } else { 0x0u8 };
		let mut new_genes = self.dna.to_vec();
		for i in 0..len / 8 {
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

		debug!(
			"crossover at {}/{}: {} * {} -> {}",
			byte,
			bit,
			self.dna.to_base64(base64::STANDARD),
			other.to_base64(base64::STANDARD),
			new_genes.to_base64(base64::STANDARD)
		);

		Genome::new(new_genes)
	}

	pub fn mutate<R: rand::Rng>(&self, rng: &mut R) -> Self {
		let mut new_genes = self.dna.to_vec();
		let n_mutations = rng.gen::<usize>() % (new_genes.len() / 8 + 1);
		for _ in 0..n_mutations {
			let (byte, bit) = split_bit(rng.gen::<usize>() % self.bit_count);
			new_genes[byte] ^= 1 << bit;
		}
		Genome::new(new_genes)
	}

	pub fn dna_cloned(&self) -> Box<[u8]> { self.dna.clone() }
}

impl fmt::Display for Genome {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.dna.to_base64(base64::STANDARD)) }
}

const BITS_FOR_FLOAT: u8 = 10;

impl Generator for Genome {
	fn next_float<T>(&mut self, min: T, max: T) -> T
	where T: rand::Rand + num::Float {
		let u0 = self.next_bits(BITS_FOR_FLOAT);
		let n: T = T::from(u0).unwrap() / T::from(1 << BITS_FOR_FLOAT).unwrap();
		n * (max - min) + min
	}

	fn next_integer<T>(&mut self, min: T, max: T) -> T
	where T: rand::Rand + num::Integer + num::ToPrimitive + num::FromPrimitive + Copy {
		num::NumCast::from(min)
			.and_then(|a| num::NumCast::from(max).map(|b| self.next_i32(a, b)))
			.and_then(num::FromPrimitive::from_i32)
			.unwrap_or(min)
	}
}

#[cfg(test)]
mod tests {}
