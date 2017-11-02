use portaudio as pa;
use pitch_calc::{Letter, LetterOctave};
use synth::Synth;
use sample;
use std;

// Currently supports i8, i32, f32.
pub type AudioSample = f32;
pub type Input = AudioSample;
pub type Output = AudioSample;

const CHANNELS: i32 = 2;
const FRAMES: u32 = 64;
const SAMPLE_HZ: f64 = 48_000.0;

pub trait SoundSystem {}

pub struct PortaudioSoundSystem {
    pub portaudio: Option<pa::PortAudio>,
    pub init_status: pa::Error,
}

impl SoundSystem for PortaudioSoundSystem {}

impl PortaudioSoundSystem {
    pub fn new() -> Self {
        match Self::init_portaudio() {
            Ok(portaudio) => PortaudioSoundSystem {
                portaudio: Some(portaudio),
                init_status: pa::Error::NoError,
            },
            Err(init_status) => PortaudioSoundSystem {
                portaudio: None,
                init_status,
            }
        }
    }

    fn init_portaudio() -> Result<pa::PortAudio, pa::Error> {
        // Construct our fancy Synth!

        // Construct PortAudio and the stream.
        let pa = pa::PortAudio::new()?;

        println!("Detected {:?} devices", pa.device_count());
//        let settings = pa.default_output_stream_settings::<f32>(
//            CHANNELS,
//            SAMPLE_HZ,
//            FRAMES,
//        )?;
//        let mut stream = pa.open_non_blocking_stream(settings, callback)?;
//        stream.start()?;
//
//        // Loop while the stream is active.
//        while let Ok(true) = stream.is_active() {
//            std::thread::sleep(std::time::Duration::from_millis(16));
//        }

        Ok(pa)
    }
}
