use backend::obj;
use core::math::Mix;
use core::color::Rgba;
use core::color::Fade;
use core::geometry::{Transform, Motion, Position, Velocity};
use core::clock::Seconds;

enum Shape {
	Round,
	Spark,
}

#[derive(Copy, Clone)]
pub enum EmitterAttachment {
	None,
	Agent(obj::Id),
	Segment(obj::Id, u8),
	Bone(obj::Id, u8, u8),
}

impl Default for EmitterAttachment {
	fn default() -> EmitterAttachment {
		EmitterAttachment::None
	}
}

pub enum EmitterStyle {
	Explosion { cluster_size: u8 },
}

impl Default for EmitterStyle {
	fn default() -> EmitterStyle {
		EmitterStyle::Explosion { cluster_size: 10u8 }
	}
}

#[derive(Default)]
pub struct Emitter {
	pub id: Option<obj::Id>,
	pub transform: Transform,
	pub motion: Motion,
	pub attached_to: EmitterAttachment,
	pub style: EmitterStyle,
}

pub struct Particle {
	transform: Transform,
	direction: Velocity,
	trail: Box<[Position]>,
	faders: Box<[f32]>,
	shape: Shape,
}

pub fn new_spore_emitter(transform: Transform) -> Emitter {
	Emitter {
		transform,
		..Emitter::default()
	}
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

	pub fn scale(&self) -> f32 {
		self.faders.get(2).unwrap_or(&1.).mix(0.5, 2.)
	}

	pub fn color(&self, index: usize) -> Option<Rgba<f32>> {
		let c = [
			([400.0, 90.0, 1.0, 1.], [0., 0., 0., 0.]),
			([1.0, 1.0, 0., 1.], [0., 0., 0., 0.])];
		self.faders.get(index)
			.map(move |fader| c[index].0.fade(c[index].1, *fader))
	}
}