use std::f32::consts::*;
use core::geometry::*;
use core::geometry::Transform;
use core::color;
use app::constants::*;

pub type Rgba = color::Rgba<f32>;

pub type Id = usize;
pub type SegmentIndex = u8;
pub type BoneIndex = u8;
pub type AttachmentIndex = u8;
// pub type PhysicsHandle = Id;

#[derive(Clone)]
pub enum Shape {
	Ball { radius: f32 },
	Box { radius: f32, ratio: f32 },
	Star {
		radius: f32,
		n: u8,
		ratio1: f32,
		ratio2: f32,
	},
	Poly { radius: f32, n: i8 },
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
			&Shape::Poly { radius, .. } => radius,
			&Shape::Triangle { radius, .. } => radius,
		}
	}

	pub fn length(&self) -> usize {
		match self {
			&Shape::Ball { .. } => 12,
			&Shape::Box { .. } => 8,
			&Shape::Poly { n, .. } => n.abs() as usize * 2,
			&Shape::Star { n, .. } => n as usize * 2,
			&Shape::Triangle { .. } => 3,
		}
	}

	pub fn is_convex(&self) -> bool {
		match self {
			&Shape::Ball { .. } => true,
			&Shape::Box { .. } => true,
			&Shape::Poly { .. } => true,
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
		Shape::Ball { radius }
	}


	pub fn new_box(radius: f32, ratio: f32) -> Self {
		Shape::Box { radius, ratio }
	}

	pub fn new_star(n: u8, radius: f32, ratio1: f32, ratio2: f32) -> Self {
		assert!(n > 1);
		assert!(radius > 0.);
		assert!(ratio1 > 0.);
		assert!(ratio2 > 0.);
		assert!(ratio1 * ratio2 <= 1.);

		Shape::Star { radius, n, ratio1, ratio2 }
	}

	pub fn new_poly(n: i8, radius: f32) -> Self {
		assert!(n > 2 || n < -2);

		Shape::Poly { radius, n }
	}

	pub fn new_triangle(radius: f32, angle1: f32, angle2: f32) -> Self {
		Shape::Triangle { radius, angle1, angle2 }
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
						let (sp, cp) = p.sin_cos();
						Position::new(xunit * sp, cp)
					})
					.collect()
			}
			&Shape::Box { ratio, .. } => {
				let w2 = xunit * ratio;
				vec![
					Position::new(0., 1.),
					Position::new(w2, 1.),
					Position::new(w2, 0.),
					Position::new(w2, -1.),
					Position::new(0., -1.),
					Position::new(-w2, -1.),
					Position::new(-w2, 0.),
					Position::new(-w2, 1.),
				]
			}
			&Shape::Poly { n, .. } => {
				let phi = PI / n.abs() as f32;
				let ratio1 = f32::cos(phi);
				let ratio = &[1., ratio1.powi(n.signum() as i32)];
				(0..(2 * n.abs()))
					.map(|i| {
						let p = i as f32 * phi;
						let r = ratio[i as usize % 2];
						let (sp, cp) = p.sin_cos();
						Position::new(xunit * r * sp, r * cp)
					})
					.collect()
			}
			&Shape::Star { n, ratio1, ratio2, .. } => {
				let mut damp = 1.;
				let ratio = &[ratio1, ratio2];
				(0..(2 * n))
					.map(|i| {
						let p = i as f32 * (PI / n as f32);
						let r = f32::max(damp, 0.01); // zero is bad!
						damp *= ratio[i as usize % 2];
						let (sp, cp) = p.sin_cos();
						Position::new(xunit * r * sp, r * cp)
					})
					.collect()
			}
			&Shape::Triangle { angle1, angle2, .. } => {
				let (sa1, ca1) = angle1.sin_cos();
				let (sa2, ca2) = angle2.sin_cos();
				vec![
					Position::new(0., 1.),
					Position::new(xunit * sa1, ca1),
					Position::new(xunit * sa2, ca2),
				]
			}
		}.into_boxed_slice()
	}
}

bitflags! {
	struct MeshFlags: u8 {
		const CW       = 0x1;
		const CCW      = 0x2;
		const CONVEX   = 0x4;
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
		let winding_flags = match winding {
			Winding::CW => MeshFlags::CW,
			Winding::CCW => MeshFlags::CCW,
		};

		let shape_flags = if shape.is_convex() {
			MeshFlags::CONVEX
		} else {
			let classifier = PolygonType::classify(vertices.as_ref());
			if classifier.is_convex() { MeshFlags::CONVEX } else { MeshFlags::empty() }
		};
		Mesh {
			shape,
			flags: winding_flags | shape_flags,
			vertices,
		}
	}

	pub fn vertex(&self, index: usize) -> Position {
		self.vertices[index % self.vertices.len()]
	}

	pub fn scaled_vertex(&self, index: usize) -> Position {
		self.vertex(index) * self.shape.radius()
	}

	#[inline]
	#[allow(dead_code)]
	pub fn is_convex(&self) -> bool {
		self.flags.contains(MeshFlags::CONVEX)
	}

	#[inline]
	pub fn winding(&self) -> Winding {
		if self.flags.contains(MeshFlags::CW) { Winding::CW } else { Winding::CCW }
	}
}

pub trait Identified {
	fn id(&self) -> Id;
}

pub trait Transformable {
	fn transform(&self) -> &Transform;
	fn transform_to(&mut self, t: &Transform);
}

#[derive(Clone)]
pub struct Material {
	pub density: f32,
	pub restitution: f32,
	pub friction: f32,
	pub linear_damping: f32,
	pub angular_damping: f32,
}

#[derive(Clone)]
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
			density: DENSITY_DEFAULT,
			restitution: RESTITUTION_DEFAULT,
			friction: FRICTION_DEFAULT,
			linear_damping: LINEAR_DAMPING_DEFAULT,
			angular_damping: ANGULAR_DAMPING,
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
	fn material(&self) -> &Material;
	fn livery(&self) -> &Livery;
}

pub trait Geometry {
	fn mesh(&self) -> &Mesh;
}

pub trait Drawable: Geometry {
	fn color(&self) -> Rgba;
}
