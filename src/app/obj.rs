use cgmath::{Matrix4, EuclideanVector, Vector2};
use std::f32::consts::*;
use std::f32::*;
use std::ops::Range;
use std::collections::HashMap;

pub type Position = Vector2<f32>;
pub type Translation = Vector2<f32>;

#[derive(Clone)]
pub struct Size {
	pub width: f32,
	pub height: f32,
}

#[derive(Clone)]
pub struct Transform {
	pub position: Position,
	pub angle: f32,
	pub scale: f32,
}

pub type Rgba = [f32; 4];
pub type Id = usize;
pub type LimbIndex = u8;
pub type BoneIndex = u8;
pub type PhysicsHandle = Id;

#[derive(Clone)]
pub enum Shape {
	Ball {
		radius: f32,
	},
	Box {
		width: f32,
		height: f32,
	},
	Star {
		// http://www.geocities.jp/nyjp07/index_asteroid_E.html
		radius: f32,
		n: u8,
		a: f32,
		b: f32,
		c: f32,
		ratio: f32,
	},
}

impl Shape {
	pub fn new_ball(r: f32) -> Shape {
		Shape::Ball { radius: r }
	}

	pub fn new_box(width: f32, height: f32) -> Shape {
		Shape::Box {
			width: width,
			height: height,
		}
	}

	pub fn new_star(radius: f32, n: u8) -> Shape {
		assert!(radius > 0.);
		assert!(n > 1);
		Shape::Star {
			radius: radius,
			n: n,
			a: 0.83255,
			b: 0.14,
			c: 1.,
			ratio: 0.5,
		}
	}

	pub fn vertices(&self) -> &[Position] {
		match *self {
			Shape::Ball { radius: r } => vec![Position { x: 0., y: r }],
			Shape::Box { width: w, height: h } => {
				let w2 = w / 2.;
				let h2 = h / 2.;
				vec![Position { x: 0., y: h2 },
					     Position { x: w2, y: h2 },
					     Position { x: w2, y: -h2 },
					     Position { x: -w2, y: -h2 },
					     Position { x: -w2, y: h2 },
					     ]
			}
			Shape::Star { radius: r, n: n, a: a, b: b, c: c, ratio: ratio } => {
				let xmax = f32::sqrt(-f32::ln(2. * f32::exp(-a * a) - 1.) / (b * b));
				let r0 = ratio * xmax;
				let rmax = r0 + (1. / c) * f32::sqrt(-f32::ln(2. * f32::exp(-a * a) - 1.)); // we want r in 0 to be 1.0

				(0..(2 * n))
					.map(|i| {
						let p = i as f32 * (PI / n as f32);
						let s = f32::sin(p * (n as f32 / 2.));
						let r = (r0 +
						         (1. / c) *
						         f32::sqrt(-f32::ln(2. * f32::exp(-a * a) - f32::exp(-b * b * xmax * xmax * s * s)))) /
						        rmax;
						Position {
							x: r * f32::sin(p), // start from (1,0), clockwise
							y: r * f32::cos(p),
						}
					})
					.collect()
			}
		}
		.as_slice()
	}
}

pub struct Mesh {
	shape: Shape,
	vertices: Vec<Position>,
}

struct GameObjectState {
	transform: Matrix4<f32>,
	physics_handle: Option<PhysicsHandle>,
}

pub struct GameObject {
	pub id: Id,
	pub state: GameObjectState,
}

pub struct Limb {
	mesh: Mesh,
	transform: Transform,
	state: GameObjectState,
}

pub struct Creature {
	id: Id,
	limbs: Vec<Limb>,
}

pub struct Flock {
	id_gen: Id,
	creatures: HashMap<Id, Creature>,
}

impl Flock {
	pub fn new() -> Flock {
		Flock {
			id_gen: 0,
			creatures: HashMap::new(),
		}
	}
}

trait Drawable {
	fn transform(&self) -> Transform;
	fn mesh(&self) -> Mesh;
	fn color(&self) -> Rgba;
}
