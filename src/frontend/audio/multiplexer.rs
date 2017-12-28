//#![feature(conservative_impl_trait)]

use dsp;
use dsp::Sample;
use dsp::Frame;
use dsp::Node;
use std::collections::HashMap;
use core::clock::Seconds;
use num;
use std::iter::Iterator;
use frontend::audio::SoundEffect;
use std::f32;

const CHANNELS: i32 = super::CHANNELS;

struct Signal<S, F> where S: num::Float {
	sample_rate: S,
	frames: Box<[F]>,
}

type StereoFrame = [f32; CHANNELS as usize];
type StereoSignal = Signal<f32, StereoFrame>;

// TODO: genericise this
struct Tone {
	pitch: f32,
	length: f32,
	amplitude: f32,
}

enum Waveform {
	Sin,
	Square(f32),
	Silence,
}

impl Waveform {
	#[inline]
	fn sample(&self, amplitude: f32, length: f32, position: f32, phase: f32) -> f32 {
		let envelope = num::clamp(1.0f32 - position / length, 0.0f32, 1.0f32);
		let value = match self {
			&Waveform::Sin => (phase * f32::consts::PI * 2.0).sin(),
			&Waveform::Square(duty_cycle) => (phase - duty_cycle).signum(),
			_ => 0.0f32,
		};
		amplitude * envelope * value
	}

	fn has_sample(&self, position: f32, length: f32) -> bool {
		match self {
			&Waveform::Sin | &Waveform::Square(_) => position < length,
			_ => false
		}
	}
}

struct Generator {
	tone: Tone,
	waveform: Waveform,
}

impl Generator {
	fn sin(pitch: f32, length: f32, amplitude: f32) -> Generator {
		Generator { tone: Tone { pitch, length, amplitude }, waveform: Waveform::Sin }
	}

	fn square(pitch: f32, length: f32, amplitude: f32) -> Generator {
		Generator { tone: Tone { pitch, length, amplitude }, waveform: Waveform::Square(0.5f32) }
	}

	fn silence() -> Generator {
		Generator { tone: Tone { pitch: 1.0f32, length: 1.0f32, amplitude: 1.0f32 }, waveform: Waveform::Silence }
	}

	//noinspection RsSelfConvention
	fn to_signal_function(self, sample_rate: f32, pan: f32) -> Box<Fn(f32) -> StereoFrame> {
		let c_pan: StereoFrame = [1.0f32 - pan, pan];
		Box::new(move |position| {
			let val = self.sample(position, position);
			dsp::Frame::from_fn(|channel| (val * c_pan[channel]).to_sample())
		})
	}

	#[inline]
	fn sample(&self, position: f32, phase: f32) -> f32 {
		self.waveform.sample(self.tone.amplitude, self.tone.length, position, phase)
	}
	#[inline]
	fn pitch(&self, position: f32) -> f32 {
		self.tone.pitch
	}
	#[inline]
	fn has_sample(&self, position: f32) -> bool {
		self.waveform.has_sample(position, self.tone.length)
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
	sample_rate: f64,
	sample_table: HashMap<SoundEffect, StereoSignal>,
	graph: dsp::Graph<[f32; CHANNELS as usize], DspNode>,
}

impl Multiplexer {
	pub fn new(sample_rate: f64) -> Multiplexer {
		// Construct our dsp graph.
		let mut graph = dsp::Graph::new();

		let mixer = graph.add_node(DspNode::Mixer(0.5f32));
		const VOICES: usize = 8;
		for index in 0..VOICES {
			graph.add_input(DspNode::new_voice(index, Generator::silence()), mixer);
		}
		// graph.set_master(Some(volume));
		graph.set_master(Some(mixer));

		let mut sample_table = HashMap::new();

		fn from_generator(generator: Generator, sample_rate: f32, pan: f32) -> StereoSignal {
			let duration = Seconds::new(generator.tone.length as f64);
			let f: Box<Fn(f32) -> StereoFrame> = generator.to_signal_function(sample_rate, pan);
			Signal::new(sample_rate, duration, f)
		}

		sample_table.insert(SoundEffect::Click(1), from_generator(Generator::square(880.0f32, 0.1f32, 0.1f32), sample_rate as f32, 0.8f32));
		sample_table.insert(SoundEffect::UserOption, from_generator(Generator::square(1000.0f32, 0.1f32, 0.1f32), sample_rate as f32, 0.6f32));
		sample_table.insert(SoundEffect::Fertilised, from_generator(Generator::sin(300.0f32, 0.3f32, 0.1f32), sample_rate as f32, 0.6f32));
		sample_table.insert(SoundEffect::NewSpore, from_generator(Generator::sin(150.0f32, 0.3f32, 0.1f32), sample_rate as f32, 0.3f32));
		sample_table.insert(SoundEffect::NewMinion, from_generator(Generator::sin(600.0f32, 0.5f32, 0.1f32), sample_rate as f32, 0.55f32));
		sample_table.insert(SoundEffect::DieMinion, from_generator(Generator::sin(90.0f32, 1.0f32, 0.2f32), sample_rate as f32, 0.1f32));

		Multiplexer {
			sample_rate,
			sample_table,
			graph,
		}
	}

	pub fn audio_requested(&mut self, buffer: &mut [[f32; CHANNELS as usize]]) {
		self.graph.audio_requested(buffer, self.sample_rate)
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
			SoundEffect::Click(1) => (Generator::square(880.0f32, 0.1f32, 0.1f32), 0.8f32),
			//SoundEffect::Release(_) => Some((400.0f32, 0.15f32, 0.3f32)),
			SoundEffect::UserOption => (Generator::square(1000.0f32, 0.1f32, 0.1f32), 0.6f32),
			SoundEffect::Fertilised => (Generator::sin(300.0f32, 0.3f32, 0.1f32), 0.6f32),
			SoundEffect::NewSpore => (Generator::sin(150.0f32, 0.3f32, 0.1f32), 0.3f32),
			SoundEffect::NewMinion => (Generator::sin(600.0f32, 0.5f32, 0.1f32), 0.55f32),
			SoundEffect::DieMinion => (Generator::sin(90.0f32, 1.0f32, 0.2f32), 0.1f32),
			_ => (Generator::silence(), 0.5f32)
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

impl<S, F> Signal<S, F> where S: num::Float + Into<f64> {
	fn new<V>(sample_rate: S, duration: Seconds, f: Box<V>) -> Signal<S, F>
		where V: Fn(S) -> F + ? Sized {
		let samples: usize = ((S::from(duration.get()).unwrap() / sample_rate).round().into()) as usize;
		let frames = (0..samples)
			.map(|i| S::from(i).unwrap() / sample_rate)
			.map(|t| f(t)).collect::<Vec<F>>();
		Signal {
			sample_rate,
			frames: frames.into_boxed_slice(),
		}
	}

	fn duration(&self) -> Seconds {
		Seconds::new(self.sample_rate.into() * self.frames.len() as f64)
	}

	fn sample_rate(&self) -> S {
		self.sample_rate
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
						*phase = (*phase + dp).fract();
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
