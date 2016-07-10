use cgmath::{Matrix4, EuclideanVector, Vector2};
use std::f32::consts::*;
use std::f32::*;

pub type Position = Vector2<f32>;
pub type Translation = Vector2<f32>;

pub struct Size {
	pub width: f32,
	pub height: f32,
}

pub struct Transform {
	pub position: Position,
	pub angle: f32,
	pub scale: f32,
}

pub type Rgba = [f32; 4];
pub type Id = usize;
pub type PhysicsHandle = Id;

pub enum Shape {
	Ball {
		radius: f32,
	},
	Box {
		size: Size,
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

use std::ops::Range;

impl Shape {
	pub fn new_ball(r: f32) -> Shape {
		Shape::Ball { radius: r }
	}

	pub fn new_box(width: f32, height: f32) -> Shape {
		Shape::Box {
			size: Size {
				width: width,
				height: height,
			},
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
				Shape::Box { size: size } => {
					let w2 = size.width / 2.;
					let h2 = size.height / 2.;
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
					let s0 = 0.;
					let rmax = r0 + (1. / c) * f32::sqrt(-f32::ln(2. * f32::exp(-a * a) - 1.)); // we want r in 0 to be 1.0

					(0..(2 * n))
						.map(|i| {
							let p = i as f32 * (PI / n as f32);
							let s = f32::sin(p * (n as f32 / 2.));
							let r = (r0 +
							         (1. / c) *
							         f32::sqrt(-f32::ln(2. * f32::exp(-a * a) -
							                            f32::exp(-b * b * xmax * xmax * s * s)))) / rmax;
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

struct GameObjectState {
	transform: Matrix4<f32>,
	physics_handle: Option<PhysicsHandle>,
}

pub struct GameObject {
	pub id: Id,
	pub shape: Shape,
	pub state: GameObjectState,
}

pub struct Limb {
	shape: Shape,
	transform: Transform,
}

pub struct Creature {
	pub id: Id,
	pub limbs: Vec<Limb>,
}

trait Drawable {
	fn transform(&self) -> Transform;
	fn shape(&self) -> Shape;
	fn color(&self) -> Rgba;
}
