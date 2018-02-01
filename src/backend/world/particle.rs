use backend::obj;
use core::color::Rgba;
use core::geometry::Transform;
use core::geometry::Position;
use core::geometry::Velocity;
use core::clock::Seconds;

enum Shape {
	Round,
	Spark,
}

pub struct Emitter {
	id: usize,
	transform: Transform,
	attached_to: Option<obj::Id>,
	phase: Seconds,
	rate: Seconds,
	cluster_size: usize,
}

pub struct Particle {
	transform: Transform,
	direction: Velocity,
	trail: Box<[Position]>,
	shape: Shape,
}

impl Particle {
	pub fn new(transform: Transform, direction: Velocity, trail: Box<[Position]>) -> Particle {
		Particle {
			transform,
			direction,
			trail,
			shape: Shape::Round,
		}
	}

	pub fn transform(&self) -> Transform {
		self.transform.clone()
	}

	pub fn color(&self) -> Rgba<f32> {
		[10., 10., 10., 1.]
	}
}