use backend::obj;
use app::constants::*;
use core::clock::Seconds;
use core::math::Mix;
use core::color::Rgba;
use core::color::Fade;
use core::geometry::{Transform, Motion, Position, Velocity};

enum Shape {
	Round(f32),
	Spark(f32),
}

enum Fader {
	Color = 0,
	Intensity = 1,
	Effect = 2,
	Scale = 3,
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
	Explosion {
		cluster_size: u8,
		color0: Rgba<f32>,
		color1: Rgba<f32>,
	},
}

impl Default for EmitterStyle {
	fn default() -> EmitterStyle {
		EmitterStyle::Explosion {
			cluster_size: 10u8,
			color0: [400., 90., 1., 1.],
			color1: [0.; 4],
		}
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
	color0: Rgba<f32>,
	color1: Rgba<f32>,
	age: Seconds,
	shape: Shape,
}

pub fn new_spore_emitter(transform: Transform) -> Emitter {
	Emitter {
		transform,
		..Emitter::default()
	}
}

impl Particle {
	pub fn round(transform: Transform,
				 direction: Velocity,
				 trail: Box<[Position]>,
				 faders: Box<[f32]>,
				 color0: Rgba<f32>,
				 color1: Rgba<f32>,
				 age: Seconds) -> Particle {
		Particle {
			transform,
			direction,
			trail,
			faders,
			color0,
			color1,
			age,
			shape: Shape::Round(DEFAULT_RIPPLE_FREQUENCY),
		}
	}

	pub fn spark(transform: Transform,
				 direction: Velocity,
				 trail: Box<[Position]>,
				 faders: Box<[f32]>,
				 color0: Rgba<f32>,
				 color1: Rgba<f32>,
				 age: Seconds) -> Particle {
		Particle {
			transform,
			direction,
			trail,
			faders,
			color0,
			color1,
			age,
			shape: Shape::Spark(DEFAULT_SPARK_RATIO),
		}
	}


	pub fn transform(&self) -> Transform {
		self.transform.clone()
	}

	pub fn scale(&self) -> f32 {
		self.faders.get(Fader::Scale as usize)
			.unwrap_or(&1.).mix(0.5, 2.)
	}

	pub fn color(&self) -> Rgba<f32> {
		self.faders.get(Fader::Color as usize)
			.map(move |fader| self.color0.fade(self.color1, *fader))
			.unwrap_or(COLOR_WHITE)
	}

	pub fn effect(&self) -> Rgba<f32> {
		let frequency = match self.shape {
			Shape::Round(frequency) => frequency,
			Shape::Spark(_) => 1.,
		};
		let ratio = match self.shape {
			Shape::Round(_) => 1.,
			Shape::Spark(ratio) => ratio,
		};

		[*self.faders.get(Fader::Intensity as usize)
			.unwrap_or(&1.0),
			self.age.get() as f32,
			frequency,
			ratio
		]
	}
}