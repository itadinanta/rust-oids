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
	Triangle {
		radius: f32,
		alpha1: f32,
		alpha2: f32,
	},
}

#[derive(Clone, Copy)]
pub enum Winding {
	CW,
	CCW,
}

impl Shape {
	pub fn new_ball(r: f32) -> Self {
		Shape::Ball { radius: r }
	}

	pub fn new_box(width: f32, height: f32) -> Self {
		Shape::Box {
			width: width,
			height: height,
		}
	}

	pub fn new_star(radius: f32, ratio: f32, n: u8) -> Self {
		assert!(radius > 0.);
		assert!(n > 1);
		Shape::Star {
			radius: radius,
			n: n,
			a: 0.83255,
			b: 0.14,
			c: 1.,
			ratio: ratio,
		}
	}

	pub fn new_triangle(radius: f32, alpha1: f32, alpha2: f32) -> Self {
		Shape::Triangle {
			radius: radius,
			alpha1: alpha1,
			alpha2: alpha2,
		}
	}

	pub fn vertices(&self, winding: Winding) -> Vec<Position> {
		let xunit = match winding {
			Winding::CW => 1.,
			Winding::CCW => -1.,
		};
		match self {
			// quarters
			&Shape::Ball { .. } => {
				// first point is always unit y
				vec![Position::new(0., 1.),
				     Position::new(xunit, 0.),
				     Position::new(0., -1.),
				     Position::new(-xunit, -1.)]
			}
			&Shape::Box { width, height } => {
				let w2 = xunit * width / height;
				// we want the first point to be unit y
				vec![Position::new(0., 1.),
				     Position::new(w2, 1.),
				     Position::new(-w2, -1.),
				     Position::new(w2, -1.),
				     Position::new(-w2, 1.)]
			}
			&Shape::Star { n, a, b, c, ratio, .. } => {
				let mut damp = 1.0;
				let xmax = f32::sqrt(-f32::ln(2. * f32::exp(-a * a) - 1.) / (b * b));
				let r0 = ratio * xmax;
				// we want r in 0 to be 1, so first point is unit y
				let rmax = r0 + (1. / c) * f32::sqrt(-f32::ln(2. * f32::exp(-a * a) - 1.));

				(0..(2 * n))
					.map(|i| {
						let p = i as f32 * (PI / n as f32);
						let s = f32::sin(p * (n as f32 / 2.));
						let r = (r0 +
						         (1. / c) *
						         f32::sqrt(-f32::ln(2. * f32::exp(-a * a) - f32::exp(-b * b * xmax * xmax * s * s)))) /
						        rmax;
						damp *= 0.9;
						Position::new(xunit * damp * r * f32::sin(p), // start from (1,0), clockwise
						              damp * r * f32::cos(p))
					})
					.collect()
			}
			&Shape::Triangle { alpha1, alpha2, .. } => {
				vec![Position::new(0., 1.),
				     Position::new(xunit * f32::sin(alpha1 * PI), f32::cos(alpha1 * PI)),
				     Position::new(xunit * f32::sin(alpha2 * PI), f32::cos(alpha2 * PI))]
			}
		}
	}
}

pub struct Mesh {
	pub shape: Shape,
	pub winding: Winding,
	pub vertices: Vec<Position>,
}

impl Mesh {
	pub fn from_shape(shape: Shape, winding: Winding) -> Self {
		let vertices = shape.vertices(winding);
		Mesh {
			shape: shape,
			winding: winding,
			vertices: vertices,
		}
	}
}

pub trait Transformable {
	fn transform(&self) -> Transform;
	fn transform_to(&mut self, t: Transform);
}

pub trait GameObject: Transformable {
	fn id(&self) -> Id;
}

#[derive(Copy, Clone)]
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

pub trait Updateable {
	fn update(&mut self, dt: f32);
}

pub trait Solid {
	fn material(&self) -> Material;
}

pub trait Geometry {
	fn mesh(&self) -> &Mesh;
}

pub trait Drawable: Geometry {
	fn color(&self) -> Rgba;
}
