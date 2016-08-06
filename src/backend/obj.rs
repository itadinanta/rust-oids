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
pub type AttachmentIndex = u8;
pub type PhysicsHandle = Id;

#[derive(Clone)]
pub enum Shape {
	Ball {
		radius: f32,
	},
	Box {
		radius: f32,
		ratio: f32,
	},
	Star {
		// http://www.geocities.jp/nyjp07/index_asteroid_E.html
		radius: f32,
		n: u8,
		ratio1: f32,
		ratio2: f32,
	},
	Triangle {
		radius: f32,
		alpha1: f32,
		alpha2: f32,
	},
}

impl Shape {
	pub fn radius(&self) -> f32 {
		match self {
			&Shape::Ball { radius } => radius,
			&Shape::Box { radius, .. } => radius,
			&Shape::Star { radius, .. } => radius,
			&Shape::Triangle { radius, .. } => radius,
		}
	}
}

#[derive(Clone, Copy)]
pub enum Winding {
	CW = 1,
	CCW = -1,
}

impl Winding {
	pub fn xunit(&self) -> f32 {
		*self as i16 as f32
	}
}

impl Shape {
	pub fn new_ball(r: f32) -> Self {
		Shape::Ball { radius: r }
	}

	pub fn new_box(width: f32, height: f32) -> Self {
		Shape::Box {
			radius: height,
			ratio: width / height,
		}
	}

	pub fn new_star(n: u8, radius: f32, ratio1: f32, ratio2: f32) -> Self {
		assert!(n > 1);
		assert!(radius > 0.);
		assert!(ratio1 > 0. && ratio1 <= 1.);
		assert!(ratio2 > 0.);
		Shape::Star {
			radius: radius,
			n: n,
			ratio1: ratio1,
			ratio2: f32::min(1. / ratio1, ratio2),
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
		let xunit = winding.xunit();
		match self {
			// first point is always unit y
			&Shape::Ball { .. } => {
				let n = 12usize;
				(0..n)
					.map(|i| {
						let p = (i as f32) / (n as f32) * 2. * PI;
						Position::new(xunit * f32::sin(p), f32::cos(p))
					})
					.collect()
			}
			&Shape::Box { ratio, .. } => {
				let w2 = xunit * ratio;
				vec![Position::new(0., 1.),
				     Position::new(w2, 1.),
				     Position::new(-w2, -1.),
				     Position::new(w2, -1.),
				     Position::new(-w2, 1.)]
			}
			&Shape::Star { n, ratio1, ratio2, .. } => {
				let mut damp = 1.;
				let ratio = &[ratio1, ratio2];
				(0..(2 * n))
					.map(|i| {
						let p = i as f32 * (PI / n as f32);
						let r = f32::max(damp, 0.2);
						damp *= ratio[i as usize % 2];
						Position::new(xunit * r * f32::sin(p), r * f32::cos(p))
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
			restitution: 0.6,
			friction: 0.7,
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
