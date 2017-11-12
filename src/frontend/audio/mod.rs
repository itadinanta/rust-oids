mod multiplexer;

use portaudio as pa;
use sample;
use dsp;
use std::thread;
use thread_priority::*;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::mpsc::SendError;
use std::sync::Arc;
use std::sync::Mutex;
use app;
use dsp::{Graph, Frame, Node, FromSample, Sample};
use dsp::sample::ToFrameSliceMut;
use backend::world::AlertEvent;
use frontend::ui::AlertPlayer;

// Currently supports i8, i32, f32.
//pub type AudioSample = f32;
//pub type Input = AudioSample;
//pub type Output = AudioSample;

const CHANNELS: i32 = 2;
const FRAMES: u32 = 64;
const SAMPLE_HZ: f64 = 48_000.0;

#[derive(Clone, Debug, Copy)]
pub enum Error {
	SystemInit,
	SynthInit,
	EventSend(SoundEffect),
}

#[derive(Clone, Debug, Copy)]
pub enum SoundEffect {
	Pitch(usize),
	Eof,
}

impl From<pa::Error> for self::Error {
	fn from(err: pa::Error) -> Self {
		match err {
			_ => self::Error::SystemInit,
		}
	}
}

impl From<SendError<SoundEffect>> for self::Error {
	fn from(err: SendError<SoundEffect>) -> Self {
		match err {
			SendError(effect) => self::Error::EventSend(effect),
		}
	}
}

pub trait SoundSystem: Sized {
	fn new() -> Result<Self, self::Error>;

	fn open(&mut self) -> Result<(), self::Error>;

	fn close(&mut self) -> Result<(), self::Error>;
}

type Stream = pa::Stream<pa::NonBlocking, pa::Output<f32>>;

pub struct PortaudioSoundSystem {
	portaudio: pa::PortAudio,
	trigger: Sender<SoundEffect>,
}

pub struct SoundSystemAlertPlayer<S> where S: SoundSystem {
	sound_system: S,
}

impl<S> SoundSystemAlertPlayer<S> where S: SoundSystem {
	fn open(&mut self) -> Result<(), Error> {
		self.sound_system.open()
	}

	pub fn close(&mut self) -> Result<(), Error> {
		self.sound_system.close()
	}
}

pub type PortaudioAlertPlayer = SoundSystemAlertPlayer<PortaudioSoundSystem>;

impl AlertPlayer<AlertEvent> for SoundSystemAlertPlayer<PortaudioSoundSystem> {
	fn play(&mut self, alert: &AlertEvent) {
		let note = match alert {
			UserClick => SoundEffect::Pitch(1),
			NewMinion => SoundEffect::Pitch(2),
			NewSpore => SoundEffect::Pitch(3),
			NewResource => SoundEffect::Pitch(4),
			DieMinion => SoundEffect::Pitch(5),
			DieResource => SoundEffect::Pitch(6),
		};
		println!("Playing alert: {:?}", alert.alert);
		self.sound_system.trigger.send(note);
	}
}

impl AlertPlayer<app::Event> for SoundSystemAlertPlayer<PortaudioSoundSystem> {
	fn play(&mut self, event: &app::Event) {
		let note = match event {
			_ => SoundEffect::Pitch(100),
		};
		let note_velocity = 1.0;
		println!("Playing event: {:?}", event);
		self.sound_system.trigger.send(note);
	}
}

impl<S> Drop for SoundSystemAlertPlayer<S> where S: SoundSystem {
	fn drop(&mut self) {
		self.close().expect("Could not stop audio system");
	}
}

impl PortaudioAlertPlayer {
	pub fn new(s: PortaudioSoundSystem) -> PortaudioAlertPlayer {
		let mut player = PortaudioAlertPlayer {
			sound_system: s,
		};
		player.open().expect("Could not open sound system");
		player
	}
}


/// Our Node to be used within the Graph.
enum DspNode {
	Synth(f64),
	Volume(f32),
}

/// Implement the `Node` trait for our DspNode.
impl Node<[f32; CHANNELS as usize]> for DspNode {
	fn audio_requested(&mut self, buffer: &mut [[f32; CHANNELS as usize]], sample_hz: f64) {
		match *self {
			DspNode::Synth(ref mut phase) => {
				dsp::slice::map_in_place(buffer, |_| {
					let val = sine_wave(*phase);
					const SYNTH_HZ: f64 = 110.0;
					*phase += SYNTH_HZ / sample_hz;
					Frame::from_fn(|_| val)
				})
			}
			DspNode::Volume(vol) => {
				dsp::slice::map_in_place(buffer, |f|
					f.map(|s| s.mul_amp(vol)))
			}
		}
	}
}

/// Return a sine wave for the given phase.
fn sine_wave<S: Sample>(phase: f64) -> S
	where
		S: Sample + FromSample<f32>,
{
	use std::f64::consts::PI;
	((phase * PI * 2.0).sin() as f32).to_sample::<S>()
}

impl SoundSystem for PortaudioSoundSystem {
	fn new() -> Result<Self, self::Error> {
		let portaudio = Self::init_portaudio()?;
		let settings = portaudio.default_output_stream_settings::<f32>(
			CHANNELS,
			SAMPLE_HZ,
			FRAMES,
		)?;
		let (tx, rx) = channel();
		// Construct our dsp graph.
		let mut graph = Graph::new();
		let synth = graph.add_node(DspNode::Synth(0.0));
		let (_, volume) = graph.add_output(synth, DspNode::Volume(1.0));
		let dsp = Arc::new(Mutex::new(graph));
		let dsp_handle = dsp.clone();

		let callback = move |pa::OutputStreamCallbackArgs { buffer, .. }| {
			let buffer: &mut [[f32; CHANNELS as usize]] =
				buffer.to_frame_slice_mut().unwrap();
			sample::slice::equilibrium(buffer);
			// uhm what?
			let mut graph = dsp_handle.lock().unwrap();
			graph.audio_requested(buffer, SAMPLE_HZ as f64);
			pa::Continue
		};
		let mut stream = portaudio.open_non_blocking_stream(settings, callback)
			.expect("Unable to open audio stream, failure in audio thread");
		let sound_thread = thread::spawn(move || {
			let thread_id = thread_native_id();
			assert!(set_thread_priority(thread_id,
										ThreadPriority::Max,
										ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Normal)).is_ok());

			info!("Started sound control thread");
			stream.start().expect("Unable to start audio stream");
			loop {
				match rx.recv() {
					Ok(SoundEffect::Eof) => break,
					Ok(alert) => {
						dsp.lock().unwrap();
						// update synth parameters here
					}
					Err(_) => break
				}
			}
			stream.close().expect("Unable to stop audio stream");
			info!("Terminated sound control thread");
		});

		Ok(PortaudioSoundSystem {
			portaudio,
			trigger: tx,
		})
	}

	fn open(&mut self) -> Result<(), self::Error> {
		Ok(())
	}

	fn close(&mut self) -> Result<(), self::Error> {
		self.trigger.send(SoundEffect::Eof).ok();
		Ok(())
	}
}

impl PortaudioSoundSystem {
	fn init_portaudio() -> Result<pa::PortAudio, pa::Error> {
		// Construct our fancy Synth!
		// Construct PortAudio and the stream.
		let pa = pa::PortAudio::new()?;
		println!("Detected {:?} devices", pa.device_count());
		Ok(pa)
	}
}
