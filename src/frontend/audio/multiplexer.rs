use dsp;
use dsp::Sample;
use dsp::Frame;
use dsp::Node;
use num;
use std::iter::Iterator;
use frontend::audio::SoundEffect;
use std::f32;

const CHANNELS: i32 = super::CHANNELS;

trait Sampling {
	fn sample(&self, position: f32, phase: f32) -> f32;
	fn pitch(&self, position: f32) -> f32;
	fn has_sample(&self, position: f32) -> bool;
}

// TODO: genericise this
struct SinWave {
	pitch: f32,
	length: f32,
	amplitude: f32,
}

struct SquareWave {
	pitch: f32,
	length: f32,
	amplitude: f32,
}

impl Sampling for SinWave {
	#[inline]
	fn sample(&self, position: f32, phase: f32) -> f32 {
		let envelope = num::clamp(1.0f32 - position / self.length, 0.0f32, 1.0f32);
		envelope * self.amplitude * (phase * f32::consts::PI * 2.0).sin()
	}
	#[inline]
	fn pitch(&self, _: f32) -> f32 {
		self.pitch
	}
	#[inline]
	fn has_sample(&self, position: f32) -> bool {
		position < self.length
	}
}

// Just to introduce a different generator
impl Sampling for SquareWave {
	#[inline]
	fn sample(&self, position: f32, phase: f32) -> f32 {
		let envelope = num::clamp(1.0f32 - position / self.length, 0.0f32, 1.0f32);
		envelope * self.amplitude * (phase * f32::consts::PI * 2.0).sin().signum() // TODO: quick and dirty and overkill. Remove sin()
	}
	#[inline]
	fn pitch(&self, _: f32) -> f32 {
		self.pitch
	}
	#[inline]
	fn has_sample(&self, position: f32) -> bool {
		position < self.length
	}
}

enum Generator {
	Sin(SinWave),
	Square(SquareWave),
	Silence,
}

impl Generator {
	fn sin(pitch: f32, length: f32, amplitude: f32) -> Generator {
		Generator::Sin(SinWave { pitch, length, amplitude })
	}
	fn square(pitch: f32, length: f32, amplitude: f32) -> Generator {
		Generator::Square(SquareWave { pitch, length, amplitude })
	}
}

impl Sampling for Generator {
	#[inline]
	fn sample(&self, position: f32, phase: f32) -> f32 {
		match self {
			&Generator::Sin(ref wave) => wave.sample(position, phase),
			&Generator::Square(ref wave) => wave.sample(position, phase),
			_ => 0.0f32,
		}
	}
	#[inline]
	fn pitch(&self, position: f32) -> f32 {
		match self {
			&Generator::Sin(ref wave) => wave.pitch(position),
			&Generator::Square(ref wave) => wave.pitch(position),
			_ => 1.0f32,
		}
	}
	#[inline]
	fn has_sample(&self, position: f32) -> bool {
		match self {
			&Generator::Sin(ref wave) => wave.has_sample(position),
			&Generator::Square(ref wave) => wave.has_sample(position),
			_ => false
		}
	}
}

enum DspNode {
	Voice {
		index: usize,
		generator: Generator,
		phase: f32,
		position: f32,
		pan: f32,
		allocated: bool,
	},
	Mixer(f32),
}

impl DspNode {
	fn new_voice(index: usize, generator: Generator) -> DspNode {
		DspNode::Voice {
			index,
			generator,
			phase: 0.0f32,
			position: 0.0f32,
			pan: 0.5f32,
			allocated: false,
		}
	}
}

pub struct Multiplexer {
	graph: dsp::Graph<[f32; CHANNELS as usize], DspNode>,
}

impl Multiplexer {
	pub fn new() -> Multiplexer {
		// Construct our dsp graph.
		let mut graph = dsp::Graph::new();

		let mixer = graph.add_node(DspNode::Mixer(0.5f32));
		const VOICES: usize = 8;
		for index in 0..VOICES {
			graph.add_input(DspNode::new_voice(index, Generator::Silence), mixer);
		}
		// graph.set_master(Some(volume));
		graph.set_master(Some(mixer));

		Multiplexer {
			graph,
		}
	}

	pub fn audio_requested(&mut self, buffer: &mut [[f32; CHANNELS as usize]], sample_hz: f64) {
		self.graph.audio_requested(buffer, sample_hz)
	}

	pub fn trigger(&mut self, effect: SoundEffect) {
		// Sweeps through completed samples
		for node in self.graph.nodes_mut() {
			match node {
				&mut DspNode::Voice { index, position, ref generator, ref mut allocated, .. } => {
					if *allocated && !generator.has_sample(position) {
						*allocated = false;
						trace!("Voice {} is available", index);
					}
				}
				_ => {}
			}
		}

		let (new_generator, new_pan) = match effect {
			SoundEffect::Click(_) => (Generator::square(880.0f32, 0.1f32, 0.1f32), 0.8f32),
			//SoundEffect::Release(_) => Some((400.0f32, 0.15f32, 0.3f32)),
			SoundEffect::UserOption => (Generator::square(1000.0f32, 0.1f32, 0.1f32), 0.6f32),
			SoundEffect::Fertilised => (Generator::sin(300.0f32, 0.3f32, 0.1f32), 0.6f32),
			SoundEffect::NewSpore => (Generator::sin(150.0f32, 0.3f32, 0.1f32), 0.3f32),
			SoundEffect::NewMinion => (Generator::sin(600.0f32, 0.5f32, 0.1f32), 0.55f32),
			SoundEffect::DieMinion => (Generator::sin(90.0f32, 1.0f32, 0.2f32), 0.1f32),
			_ => (Generator::Silence, 0.5f32)
		};
		// Finds the first available voice
		let mut free_voice = self.graph.nodes_mut().find(|n| match n {
			&&mut DspNode::Voice { allocated, .. } => !allocated,
			_ => false
		});
		// Resets the voice's phase and grabs it
		free_voice.as_mut().map(|n| {
			match n {
				&mut &mut DspNode::Voice {
					index, ref mut phase, ref mut position, ref mut pan,
					ref mut allocated, ref mut generator, ..
				} => {
					*phase = 0.0f32;
					*position = 0.0f32;
					*generator = new_generator;
					*pan = new_pan;
					*allocated = true;
					trace!("Voice {} playing", index);
				}
				_ => ()
			}
		});
	}
}

/// Our Node to be used within the Graph.
/// Implement the `Node` trait for our DspNode.
impl dsp::Node<[f32; CHANNELS as usize]> for DspNode {
	fn audio_requested(&mut self, buffer: &mut [[f32; CHANNELS as usize]], sample_hz: f64) {
		match *self {
			DspNode::Voice { ref mut phase, ref mut position, ref generator, allocated, pan, .. } => {
				dsp::slice::equilibrium(buffer);
				let c_pan: [f32; CHANNELS as usize] = [1.0f32 - pan, pan];
				if allocated {
					let dt = 1.0f32 / sample_hz as f32;
					dsp::slice::map_in_place(buffer, |_| {
						let val = generator.sample(*position, *phase);
						let dp = generator.pitch(*position) / sample_hz as f32;
						*phase += dp;
						*position += dt;
						dsp::Frame::from_fn(|channel| (val * c_pan[channel]).to_sample())
					})
				}
			}
			DspNode::Mixer(vol) => {
				dsp::slice::map_in_place(buffer, |f|
					f.map(|s| s.mul_amp(vol)))
			}
		}
	}
}
