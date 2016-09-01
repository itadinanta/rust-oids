use std::f32::consts::*;
use core::geometry::*;
use core::color;

pub type Rgba = color::Rgba<f32>;

pub type Id = usize;
pub type SegmentIndex = u8;
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
		radius: f32,
		n: u8,
		ratio1: f32,
		ratio2: f32,
	},
	Triangle {
		radius: f32,
		angle1: f32,
		angle2: f32,
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

	pub fn length(&self) -> usize {
		match self {
			&Shape::Ball { .. } => 12,
			&Shape::Box { .. } => 8,
			&Shape::Star { n, .. } => n as usize * 2,
			&Shape::Triangle { .. } => 3,
		}
	}

	pub fn is_convex(&self) -> bool {
		match self {
			&Shape::Ball { .. } => true,
			&Shape::Box { .. } => true,
			&Shape::Star { .. } => false,
			&Shape::Triangle { .. } => true,
		}
	}

	pub fn is_poly(&self) -> bool {
		match self {
			&Shape::Ball { .. } => true,
			&Shape::Box { .. } => true,
			&Shape::Star { .. } => false,
			&Shape::Triangle { .. } => true,
		}
	}

	pub fn mid(&self) -> isize {
		self.length() as isize / 2
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
	pub fn new_ball(radius: f32) -> Self {
		Shape::Ball { radius: radius }
	}


	pub fn new_box(radius: f32, ratio: f32) -> Self {
		Shape::Box {
			radius: radius,
			ratio: ratio,
		}
	}

	pub fn new_star(n: u8, radius: f32, ratio1: f32, ratio2: f32) -> Self {
		assert!(n > 1);
		assert!(radius > 0.);
		assert!(ratio1 > 0.);
		assert!(ratio2 > 0.);
		assert!(ratio1 * ratio2 <= 1.);

		Shape::Star {
			radius: radius,
			n: n,
			ratio1: ratio1,
			ratio2: ratio2,
		}
	}

	pub fn new_triangle(radius: f32, angle1: f32, angle2: f32) -> Self {
		Shape::Triangle {
			radius: radius,
			angle1: angle1,
			angle2: angle2,
		}
	}

	pub fn vertices(&self, winding: Winding) -> Box<[Position]> {
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
					     Position::new(w2, 0.),
					     Position::new(w2, -1.),
					     Position::new(0., -1.),
					     Position::new(-w2, -1.),
					     Position::new(-w2, 0.),
					     Position::new(-w2, 1.)]
				}
				&Shape::Star { n, ratio1, ratio2, .. } => {
					let mut damp = 1.;
					let ratio = &[ratio1, ratio2];
					(0..(2 * n))
						.map(|i| {
							let p = i as f32 * (PI / n as f32);
							let r = f32::max(damp, 0.01); // zero is bad!
							damp *= ratio[i as usize % 2];
							Position::new(xunit * r * f32::sin(p), r * f32::cos(p))
						})
						.collect()
				}
				&Shape::Triangle { angle1, angle2, .. } => {
					vec![Position::new(0., 1.),
					     Position::new(xunit * f32::sin(angle1), f32::cos(angle1)),
					     Position::new(xunit * f32::sin(angle2), f32::cos(angle2))]
				}
			}
			.into_boxed_slice()
	}
}

bitflags! {
	flags MeshFlags: u8 {
		const CW       = 0x1,
		const CCW      = 0x2,
		const CONVEX   = 0x4,
		const POLY     = 0x8,
	}
}

#[derive(Clone)]
pub struct Mesh {
	flags: MeshFlags,
	pub shape: Shape,
	pub vertices: Box<[Position]>,
}

impl Mesh {
	pub fn from_shape(shape: Shape, winding: Winding) -> Self {
		let vertices = shape.vertices(winding);
		let flags = match winding {
			Winding::CW => CW,
			Winding::CCW => CCW,
		} |
		            if shape.is_poly() {
			CONVEX | POLY
		} else if shape.is_convex() {
			CONVEX
		} else {
			MeshFlags::empty()
		};
		Mesh {
			shape: shape,
			flags: flags,
			vertices: vertices,
		}
	}

	#[inline]
	pub fn is_convex(&self) -> bool {
		self.flags.contains(CONVEX)
	}

	#[inline]
	pub fn is_poly(&self) -> bool {
		self.flags.contains(POLY)
	}
	#[inline]
	pub fn winding(&self) -> Winding {
		if self.flags.contains(CW) {
			Winding::CW
		} else {
			Winding::CCW
		}
	}
}

pub trait Identified {
	fn id(&self) -> Id;
}

pub trait Transformable {
	fn transform(&self) -> Transform;
	fn transform_to(&mut self, t: Transform);
}

#[derive(Copy, Clone)]
pub struct Material {
	pub density: f32,
	pub restitution: f32,
	pub friction: f32,
}

#[derive(Copy, Clone)]
pub struct Livery {
	pub albedo: Rgba,
	pub frequency: f32,
	pub phase: f32,
	pub amplitude: f32,
	pub seed: f32,
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

impl Default for Livery {
	fn default() -> Self {
		Livery {
			albedo: [1., 1., 1., 1.],
			frequency: 0.5,
			phase: 0.,
			amplitude: 0.5,
			seed: 0.,
		}
	}
}

pub trait Solid {
	fn material(&self) -> Material;
	fn livery(&self) -> Livery;
}

pub trait Geometry {
	fn mesh(&self) -> &Mesh;
}

pub trait Drawable: Geometry {
	fn color(&self) -> Rgba;
}
