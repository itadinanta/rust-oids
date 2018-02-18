use backend::obj;
use app::constants::*;
use core::clock::Seconds;
use core::color::Rgba;
use core::color::Fade;
use core::geometry::{Transform, Motion, Position, Velocity};

pub enum Fader {
	Color = 0,
	Scale = 1,
	Effect = 2,
	Frequency = 3,
	Count = 4,
}

#[derive(Copy, Clone)]
pub enum EmitterAttachment {
	None,
	Agent(obj::Id),
	Segment(obj::Id, u8),
	Vertex(obj::Id, u8, u8),
}

impl Default for EmitterAttachment {
	fn default() -> EmitterAttachment {
		EmitterAttachment::None
	}
}

pub enum EmitterStyle {
	Explosion {
		cluster_size: u8,
		color: Rgba<f32>,
	},
	Ping {
		color: Rgba<f32>,
	},
	Sparkle {
		cluster_size: u8,
		color: Rgba<f32>,
	},
}

impl Default for EmitterStyle {
	fn default() -> EmitterStyle {
		EmitterStyle::Explosion {
			cluster_size: 10u8,
			color: COLOR_SUNSHINE,
		}
	}
}

impl EmitterStyle {
	fn color_bang(color: Rgba<f32>, boost: f32) -> EmitterStyle {
		EmitterStyle::Explosion {
			cluster_size: 10u8,
			color: [
				color[0] * boost,
				color[1] * boost,
				color[2] * boost,
				color[3]],
		}
	}

	fn color_sparkle(color: Rgba<f32>, boost: f32) -> EmitterStyle {
		EmitterStyle::Sparkle {
			cluster_size: 10u8,
			color: [
				color[0] * boost,
				color[1] * boost,
				color[2] * boost,
				color[3]],
		}
	}

	fn color_ping(color: Rgba<f32>, boost: f32) -> EmitterStyle {
		EmitterStyle::Ping {
			color: [
				color[0] * boost,
				color[1] * boost,
				color[2] * boost,
				color[3]],
		}
	}
}

pub struct Particle {
	transform: Transform,
	direction: Velocity,
	trail: Box<[Position]>,
	faders: [f32; 4],
	color: (Rgba<f32>, Rgba<f32>),
	effect: (Rgba<f32>, Rgba<f32>),
	age: Seconds,
}

#[derive(Default)]
pub struct Emitter {
	pub id: Option<obj::Id>,
	pub transform: Transform,
	pub motion: Motion,
	pub attached_to: EmitterAttachment,
	pub style: EmitterStyle,
}

impl Emitter {
	pub fn for_new_spore(transform: Transform, color: Rgba<f32>, id: obj::Id) -> Emitter {
		Emitter {
			transform,
			attached_to: EmitterAttachment::Agent(id),
			style: EmitterStyle::color_ping(color, 100.),
			..Emitter::default()
		}
	}
	pub fn for_new_minion(transform: Transform, color: Rgba<f32>) -> Emitter {
		Emitter {
			transform,
			style: EmitterStyle::color_sparkle(color, 100.),
			..Emitter::default()
		}
	}
	pub fn for_dead_minion(transform: Transform, color: Rgba<f32>) -> Emitter {
		Emitter {
			transform,
			style: EmitterStyle::color_bang(color, 100.),
			..Emitter::default()
		}
	}
}

impl Particle {
	pub fn spark(frequency: f32, ratio: f32) -> [Rgba<f32>; 2] {
		[[1., 1., frequency, ratio],
			[1., 1., frequency, ratio]]
	}

	pub fn round(frequency: f32) -> [Rgba<f32>; 2] {
		[[1., 1., frequency, 1.],
			[1., 1., frequency, 1.]]
	}

	pub fn new(transform: Transform,
			   direction: Velocity,
			   trail: Box<[Position]>,
			   faders: [f32; 4],
			   color: (Rgba<f32>, Rgba<f32>),
			   effect: (Rgba<f32>, Rgba<f32>),
			   age: Seconds) -> Particle {
		Particle {
			transform,
			direction,
			trail,
			faders,
			color,
			effect,
			age,
		}
	}

	pub fn transform(&self) -> Transform {
		self.transform.clone()
	}

	pub fn scale(&self) -> f32 {
		self.faders[Fader::Scale as usize]
	}

	pub fn color(&self) -> Rgba<f32> {
		self.color.0.fade(self.color.1, self.faders[Fader::Color as usize])
	}

	pub fn effect(&self) -> Rgba<f32> {
		let effect = self.effect.0.fade(self.effect.1, self.faders[Fader::Effect as usize]);
		let frequency = self.faders[Fader::Frequency as usize];
		[effect[0], effect[1] * self.age.get() as f32, effect[2] * frequency, effect[3]]
	}
}