mod multiplexer;

use portaudio as pa;
use sample;
use std::thread;
#[cfg(unix)]
use thread_priority::*;
use std::io;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::mpsc::SendError;
use std::sync::Arc;
use std::sync::Mutex;
use app;
use sample::ToFrameSliceMut;
use frontend::ui::AlertPlayer;
use backend::world::Alert;
// Currently supports i8, i32, f32.
//pub type AudioSample = f32;
//pub type Input = AudioSample;
//pub type Output = AudioSample;

const CHANNELS: usize = 2;
const SAMPLE_HZ: f64 = 48000.0;
const FRAMES: u32 = 200;
const MAX_VOICES: usize = 64;

#[allow(unused)]
#[derive(Clone, Debug, Copy)]
pub enum Error {
	SystemInit,
	SynthInit,
	EventSend(SoundEffect),
	ThreadJoin,
}

#[allow(unused)]
#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
pub enum SoundEffect {
	Startup,
	Bullet(usize),
	Click(usize),
	Release(usize),
	NewSpore,
	NewMinion,
	GrowMinion,
	DieMinion,
	SelectMinion,
	UserOption,
	Fertilised,
	Eat,
	Eof,
	None,
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

impl From<io::Error> for self::Error {
	fn from(err: io::Error) -> Self {
		match err {
			_ => self::Error::ThreadJoin,
		}
	}
}

pub trait SoundSystem: Sized {
	fn new() -> Result<Self, self::Error>;

	fn open(&mut self) -> Result<(), self::Error>;

	fn close(&mut self) -> Result<(), self::Error>;
}

#[allow(unused)]
pub struct ThreadedSoundSystem {
	sound_thread: Option<thread::JoinHandle<()>>,
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

pub type ThreadedAlertPlayer = SoundSystemAlertPlayer<ThreadedSoundSystem>;

impl AlertPlayer<Alert, self::Error> for SoundSystemAlertPlayer<ThreadedSoundSystem> {
	fn play(&mut self, alert: &Alert) -> Result<(), self::Error> {
		let note = match alert {
			&Alert::BeginSimulation => SoundEffect::Startup,
			&Alert::NewMinion => SoundEffect::NewMinion,
			&Alert::NewSpore => SoundEffect::NewSpore,
			&Alert::Fertilised => SoundEffect::Fertilised,
			&Alert::DieMinion => SoundEffect::DieMinion,
			&Alert::GrowMinion => SoundEffect::GrowMinion,
			&Alert::NewBullet(id) => SoundEffect::Bullet(id),
			_ => SoundEffect::None,
		};
		trace!("Playing alert: {:?}", alert);
		self.sound_system.trigger.send(note)?;
		Ok(())
	}
}

impl AlertPlayer<app::Event, self::Error> for SoundSystemAlertPlayer<ThreadedSoundSystem> {
	fn play(&mut self, event: &app::Event) -> Result<(), self::Error> {
		use app::Event;
		let sound_effect = match event {
			&Event::CamReset |
			&Event::NextLight |
			&Event::PrevLight |
			&Event::NextBackground |
			&Event::PrevBackground |
			&Event::NextSpeedFactor |
			&Event::PrevSpeedFactor |
			&Event::Reload |
			&Event::SaveGenePoolToFile |
			&Event::SaveWorldToFile |
			&Event::DeselectAll |
			&Event::ToggleDebug => SoundEffect::UserOption,

			&Event::PickMinion(_) => SoundEffect::SelectMinion,

			&Event::NewMinion(_) |
			&Event::RandomizeMinion(_) => SoundEffect::NewMinion,

			&Event::EndDrag(_, _, _) => SoundEffect::Release(0),

			_ => SoundEffect::None,
		};
		trace!("Playing event: {:?}", event);
		self.sound_system.trigger.send(sound_effect)?;
		Ok(())
	}
}

impl<S> Drop for SoundSystemAlertPlayer<S> where S: SoundSystem {
	fn drop(&mut self) {
		self.close().expect("Could not stop audio system");
	}
}

impl ThreadedAlertPlayer {
	pub fn new(sound_system: ThreadedSoundSystem) -> Self {
		let mut player = ThreadedAlertPlayer {
			sound_system,
		};
		player.open().expect("Could not open sound system");
		player
	}
}

impl SoundSystem for ThreadedSoundSystem {
	fn new() -> Result<ThreadedSoundSystem, self::Error> {
		let (tx, rx) = channel();
		let sound_thread = thread::Builder::new().name("SoundControl".to_string()).spawn(move || {
			info!("Started sound control thread");
			let portaudio = pa::PortAudio::new()
				.expect("Unable to open portAudio");
			if let Ok(device_count) = portaudio.device_count() {
				info!("Detected {:?} devices", device_count);
				if let Ok(default_device) = portaudio.default_output_device() {
					if let Ok(devices) = portaudio.devices() {
						for device_result in devices {
							if let Ok((device_index, device_info)) = device_result {
								if device_index == default_device {
									info!("Using device {:?} {}", device_index, device_info.name);
								} else {
									info!("Found device {:?} {}", device_index, device_info.name);
								}
							}
						}
					}
				}
			}
			let settings = portaudio.default_output_stream_settings::<f32>(
				CHANNELS as i32,
				SAMPLE_HZ,
				FRAMES,
			).expect("Unable to setup portAudio");

			let dsp = Arc::new(Mutex::new(multiplexer::Multiplexer::new(SAMPLE_HZ, MAX_VOICES)));
			let dsp_handle = dsp.clone();

			let callback = move |pa::OutputStreamCallbackArgs { buffer, .. }| {
				trace!("Callback start");
				let buffer: &mut [[f32; CHANNELS as usize]] =
					buffer.to_frame_slice_mut().unwrap();
				sample::slice::equilibrium(buffer);
				// uhm what?
				dsp_handle.lock().unwrap().audio_requested(buffer);

				trace!("Callback end");
				pa::Continue
			};
			let mut stream = portaudio.open_non_blocking_stream(settings, callback)
				.expect("Unable to open audio stream, failure in audio thread");
			#[cfg(unix)] {
				// push up thread priority
				let thread_id = thread_native_id();
				assert!(set_thread_priority(thread_id,
											ThreadPriority::Max,
											ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Normal)).is_ok());
			}
			stream.start().expect("Unable to start audio stream");
			'sound_main: loop {
				match rx.recv() {
					Ok(SoundEffect::Eof) => {
						info!("Requested termination, exiting");
						break 'sound_main;
					}
					Ok(sound_effect) => {
						dsp.lock().unwrap().trigger(sound_effect)
					}
					Err(msg) => {
						warn!("Received error {:?}", msg);
						break 'sound_main;
					}
				}
			}
			info!("Closing audio stream");
			#[cfg(unix)] {
				// push up thread priority
				let thread_id = thread_native_id();
				assert!(set_thread_priority(thread_id,
											ThreadPriority::Specific(0),
											ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Normal)).is_ok());
			}
			match stream.close() {
				Err(msg) => error!("Unable to close audio stream: {:?}", msg),
				Ok(_) => info!("Close audio stream"),
			}
			info!("Terminating portaudio system");
			portaudio.terminate().expect("Unable to terminate portaudio session");
			info!("Terminated sound control thread");
		})?;

		Ok(ThreadedSoundSystem {
			sound_thread: Some(sound_thread),
			trigger: tx,
		})
	}

	fn open(&mut self) -> Result<(), self::Error> {
		Ok(())
	}

	fn close(&mut self) -> Result<(), self::Error> {
		self.trigger.send(SoundEffect::Eof).ok();
		let result = self.sound_thread.take().unwrap().join();
		match result {
			Ok(_) => Ok(()),
			Err(_) => Err(self::Error::ThreadJoin),
		}
	}
}