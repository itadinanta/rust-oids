use dsp;
use dsp::Sample;
use dsp::Frame;
use dsp::Node;
use std::iter::Iterator;
use frontend::audio::SoundEffect;
use std::f32;

const CHANNELS: i32 = 2;

enum DspNode {
	Voice {
		index: usize,
		pitch: f32,
		position: f32,
		length: f32,
		phase: f32,
		amplitude: f32,
		pan: f32,
		allocated: bool,
	},
	Mixer(f32),
}

impl DspNode {
	fn new_voice(index: usize, pitch: f32) -> DspNode {
		DspNode::Voice {
			index,
			pitch,
			position: 0.0f32,
			length: 0.2f32,
			phase: 0.0f32,
			amplitude: 1.0f32,
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
			graph.add_input(DspNode::new_voice(index, (index as f32 * 44.0f32 + 110.0f32)), mixer);
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
				&mut DspNode::Voice { index, position, length, ref mut allocated, .. } => {
					if *allocated && position >= length {
						*allocated = false;
						info!("Voice {} is available", index);
					}
				}
				_ => {}
			}
		}

		if let Some((new_pitch, new_length, new_amplitude, new_pan)) = match effect {
			SoundEffect::Click(_) => Some((880.0f32, 0.1f32, 0.1f32, 0.8f32)),
			//SoundEffect::Release(_) => Some((400.0f32, 0.15f32, 0.3f32)),
			SoundEffect::UserOption => Some((1000.0f32, 0.1f32, 0.1f32, 0.6f32)),
			SoundEffect::NewSpore => Some((300.0f32, 0.3f32, 0.1f32, 0.3f32)),
			SoundEffect::NewMinion => Some((220.0f32, 0.5f32, 0.1f32, 0.55f32)),
			SoundEffect::DieMinion => Some((90.0f32, 1.0f32, 0.2f32, 0.1f32)),
			_ => None
		} {
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
						ref mut allocated, ref mut pitch, ref mut amplitude, ref mut length, ..
					} => {
						*phase = 0.0f32;
						*position = 0.0f32;
						*pitch = new_pitch;
						*length = new_length;
						*amplitude = new_amplitude;
						*pan = new_pan;
						*allocated = true;
						info!("Voice {} playing", index);
					}
					_ => ()
				}
			});
		}
	}
}

/// Our Node to be used within the Graph.
/// Implement the `Node` trait for our DspNode.
impl dsp::Node<[f32; CHANNELS as usize]> for DspNode {
	fn audio_requested(&mut self, buffer: &mut [[f32; CHANNELS as usize]], sample_hz: f64) {
		match *self {
			DspNode::Voice { ref mut phase, ref mut position, length, pitch, amplitude, allocated, pan, .. } => {
				dsp::slice::equilibrium(buffer);
				let c_pan: [f32; CHANNELS as usize] = [1.0f32 - pan, pan];
				if allocated {
					let dp = pitch / sample_hz as f32;
					let dt = 1.0f32 / sample_hz as f32;
					dsp::slice::map_in_place(buffer, |_| {
						let envelope = f32::max(0.0f32, 1.0f32 - *position / length);
						let val = envelope * amplitude * (*phase * f32::consts::PI * 2.0).sin();
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
