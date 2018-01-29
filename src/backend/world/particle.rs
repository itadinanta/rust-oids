use backend::obj;
use core::color::Rgba;
use core::geometry::Transform;
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
	prev_transform: Transform,
	shape: Shape,
}

impl Particle {
	pub fn new(transform: Transform, prev_transform: Transform) -> Particle {
		Particle {
			transform,
			prev_transform,
			shape: Shape::Round,
		}
	}

	pub fn transform(&self) -> Transform {
		self.transform.clone()
	}

	pub fn color(&self) -> Rgba<f32> {
		[1., 1., 1., 1.]
	}
}