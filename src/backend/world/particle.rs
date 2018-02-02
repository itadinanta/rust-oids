use backend::obj;
use core::color::Rgba;
use core::color::Fade;
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
	faders: Box<[f32]>,
	shape: Shape,
}

impl Particle {
	pub fn new(transform: Transform, direction: Velocity, trail: Box<[Position]>, faders: Box<[f32]>) -> Particle {
		Particle {
			transform,
			direction,
			trail,
			faders,
			shape: Shape::Round,
		}
	}

	pub fn transform(&self) -> Transform {
		self.transform.clone()
	}

	pub fn color(&self, index: usize) -> Option<Rgba<f32>> {
		let c = [
			([400.0, 90.0, 1.0, 1.], [0.01, 0., 0., 0.]),
			([1.0, 1.0, 0., 1.], [0., 0., 0., 0.])];
		self.faders.get(index)
			.map(move |fader| c[index].0.fade(c[index].1, *fader))
	}
}