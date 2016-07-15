use cgmath::Vector2;
use std::f32::consts::*;

pub type Position = Vector2<f32>;
pub type Translation = Vector2<f32>;

#[derive(Clone, Default)]
pub struct Size {
	pub width: f32,
	pub height: f32,
}

#[derive(Copy, Clone)]
pub struct Transform {
	pub position: Position,
	pub angle: f32,
	pub scale: f32,
}

impl Default for Transform {
	fn default() -> Transform {
		Transform {
			position: Position::new(0., 0.),
			angle: 0.,
			scale: 1.,
		}
	}
}
impl Transform {
	pub fn with_position(position: Position) -> Self {
		Transform { position: position, ..Transform::default() }
	}
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

	pub fn vertices(&self) -> Vec<Position> {
		match *self {
			Shape::Ball { radius } => vec![Position::new(0., radius)],
			Shape::Box { width, height } => {
				let w2 = width / 2.;
				let h2 = height / 2.;
				vec![Position::new(0., h2),
				     Position::new(w2, h2),
				     Position::new(-w2, -h2),
				     Position::new(w2, -h2),
				     Position::new(-w2, h2)]
			}
			Shape::Star { radius, n, a, b, c, ratio } => {
				let xmax = f32::sqrt(-f32::ln(2. * f32::exp(-a * a) - 1.) / (b * b));
				let r0 = ratio * xmax;
				let rmax = r0 + (1. / c) * f32::sqrt(-f32::ln(2. * f32::exp(-a * a) - 1.)) / radius; // we want r in 0 to be radius

				(0..(2 * n))
					.map(|i| {
						let p = i as f32 * (PI / n as f32);
						let s = f32::sin(p * (n as f32 / 2.));
						let r = (r0 +
						         (1. / c) *
						         f32::sqrt(-f32::ln(2. * f32::exp(-a * a) - f32::exp(-b * b * xmax * xmax * s * s)))) /
						        rmax;
						Position::new(r * f32::sin(p), // start from (1,0), clockwise
						              r * f32::cos(p))
					})
					.collect()
			}
		}
	}
}

pub struct Mesh {
	pub shape: Shape,
	pub vertices: Vec<Position>,
}

pub trait Transformable {
	fn transform(&self) -> Transform;
	fn transform_to(&mut self, t: Transform);
}

pub trait GameObject: Transformable {
	fn id(&self) -> Id;
}

pub struct Material {
	pub density: f32,
	pub restitution: f32,
	pub friction: f32,
}

impl Default for Material {
	fn default() -> Self {
		Material {
			density: 1.0,
			restitution: 0.2,
			friction: 0.3,
		}
	}
}

trait Geometry {
	fn transform(&self) -> Transform;
	fn mesh(&self) -> Mesh;
}

trait Solid {
	fn material(&self) -> Material;
}

trait Drawable: Geometry {
	fn color(&self) -> Rgba;
}
