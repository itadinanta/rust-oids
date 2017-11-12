use portaudio as pa;
use pitch_calc::{Letter, LetterOctave};
use synth;
use sample;
use std::thread;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;
use app;
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
}

impl From<pa::Error> for self::Error {
	fn from(err: pa::Error) -> Self {
		match err {
			_ => self::Error::SystemInit,
		}
	}
}

pub trait SoundSystem: Sized {
	fn new() -> Result<Self, self::Error>;

	fn open(&mut self) -> Result<(), self::Error>;

	fn close(&mut self) -> Result<(), self::Error>;
}

type Synth = synth::Synth<synth::instrument::mode::Poly,
	(),
	synth::oscillator::waveform::Sine,
	synth::Envelope,
	synth::Envelope,
	()>;
type Stream = pa::Stream<pa::NonBlocking, pa::Output<f32>>;

pub struct PortaudioSoundSystem {
	portaudio: pa::PortAudio,
	synth: Arc<Mutex<Synth>>,
	stream: Stream,
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
			UserClick => LetterOctave(Letter::C, 4),
			NewMinion => LetterOctave(Letter::C, 3),
			NewSpore => LetterOctave(Letter::G, 4),
			NewResource => LetterOctave(Letter::D, 5),
			DieMinion => LetterOctave(Letter::C, 3),
			DieResource => LetterOctave(Letter::E, 3),
		};
		let note_velocity = 1.0;
		println!("Playing alert: {:?}", alert.alert);
		let mut synth = self.sound_system.synth.lock().unwrap();
		synth.note_off(note);
		synth.note_on(note, note_velocity);
	}
}

impl AlertPlayer<app::Event> for SoundSystemAlertPlayer<PortaudioSoundSystem> {
	fn play(&mut self, event: &app::Event) {
		let note = match event {
			_ => LetterOctave(Letter::C, 4),
		};
		let note_velocity = 1.0;
		println!("Playing event: {:?}", event);
		let mut synth = self.sound_system.synth.lock().unwrap();
		synth.note_off(note);
		synth.note_on(note, note_velocity);
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

impl SoundSystem for PortaudioSoundSystem {
	fn new() -> Result<Self, self::Error> {
		let portaudio = Self::init_portaudio()?;
		let synth = Arc::new(Mutex::new(Self::new_synth()));

		let settings = portaudio.default_output_stream_settings::<f32>(
			CHANNELS,
			SAMPLE_HZ,
			FRAMES,
		)?;
		let synth_closure = synth.clone();
		let callback = move |pa::OutputStreamCallbackArgs { buffer, .. }| {
			let buffer: &mut [[f32; CHANNELS as usize]] =
				sample::slice::to_frame_slice_mut(buffer).unwrap();
			sample::slice::equilibrium(buffer);
			// uhm what?
			let mut synth = synth_closure.lock().unwrap();
			synth.fill_slice(buffer, SAMPLE_HZ as f64);
			pa::Continue
		};

		let stream = portaudio.open_non_blocking_stream(settings, callback)?;

		Ok(PortaudioSoundSystem {
			portaudio,
			synth,
			stream,
		})
	}

	fn open(&mut self) -> Result<(), self::Error> {
		self.stream.start()?;
		Ok(())
	}

	fn close(&mut self) -> Result<(), self::Error> {
		self.stream.stop()?;
		self.stream.close()?;
		Ok(())
	}
}

impl PortaudioSoundSystem {
	pub fn new_synth() -> Synth {
		let synth = {
			use synth::{Point, Oscillator, oscillator, Envelope};

			// The following envelopes should create a downward pitching sine wave that gradually quietens.
			// Try messing around with the points and adding some of your own!
			let amp_env = Envelope::from(vec![
				//         Time ,  Amp ,  Curve
				Point::new(0.0, 0.0, 0.0),
				Point::new(0.01, 1.0, 0.0),
				Point::new(0.45, 1.0, 0.0),
				Point::new(0.81, 0.8, 0.0),
				Point::new(1.0, 0.0, 0.0),
			]);
			let freq_env = Envelope::from(vec![
				//         Time    , Freq   , Curve
				Point::new(0.0, 0.0, 0.0),
				Point::new(0.00136, 1.0, 0.0),
				Point::new(0.015, 0.02, 0.0),
				Point::new(0.045, 0.005, 0.0),
				Point::new(0.1, 0.0022, 0.0),
				Point::new(0.35, 0.0011, 0.0),
				Point::new(1.0, 0.0, 0.0),
			]);

			// Now we can create our oscillator from our envelopes.
			// There are also Sine, Noise, NoiseWalk, SawExp and Square waveforms.
			let oscillator = Oscillator::new(oscillator::waveform::Sine, amp_env, freq_env, ());

			// Here we construct our Synth from our oscillator.
			synth::Synth::poly(())
				.oscillator(oscillator) // Add as many different oscillators as desired.
				.duration(400.0) // Milliseconds.
				.base_pitch(LetterOctave(Letter::C, 1).hz()) // Hz.
				//.loop_points(0.49, 0.51) // Loop start and end points.
				.fade(500.0, 500.0) // Attack and Release in milliseconds.
				.num_voices(16) // By default Synth is monophonic but this gives it `n` voice polyphony.
				.volume(0.2)
				.detune(0.5)
				.spread(1.0)
		};
		synth
	}

	fn background_audio(self) -> Result<Sender<AlertEvent>, Error> {
		let (tx, rx) = channel();
		thread::spawn(move || {
			let settings = self.portaudio.default_output_stream_settings::<f32>(
				CHANNELS,
				SAMPLE_HZ,
				FRAMES,
			)?;
			let synth = Arc::new(Mutex::new(Self::new_synth()));
			let synth_callback = synth.clone();
			let callback = move |pa::OutputStreamCallbackArgs { buffer, .. }| {
				let buffer: &mut [[f32; CHANNELS as usize]] =
					sample::slice::to_frame_slice_mut(buffer).unwrap();
				sample::slice::equilibrium(buffer);
				// uhm what?
				synth_callback.lock().unwrap().fill_slice(buffer, SAMPLE_HZ as f64);
				pa::Continue
			};

			let mut stream = self.portaudio.open_non_blocking_stream(settings, callback)?;

			loop {
				match rx.recv() {
					Ok(alert) => synth.lock().unwrap().note_on(LetterOctave(Letter::C, 4), 1.0f32),
					Err(_) => break
				}
			}
			stream.close()
		});

		Ok(tx)
	}

	fn init_portaudio() -> Result<pa::PortAudio, pa::Error> {
		// Construct our fancy Synth!
		// Construct PortAudio and the stream.
		let pa = pa::PortAudio::new()?;
		println!("Detected {:?} devices", pa.device_count());
		Ok(pa)
	}
}
