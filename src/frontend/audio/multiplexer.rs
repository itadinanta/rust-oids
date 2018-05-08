//#![feature(conservative_impl_trait)]

use sample;
use pitch_calc::{Letter, LetterOctave};
use std::collections::HashMap;
use bit_set::BitSet;
use core::clock::{seconds, Seconds};
use num;
use num::NumCast;
use num_traits::FloatConst;
use std::iter::Iterator;
use frontend::audio::SoundEffect;
use std::f32;
use std::f64;

const CHANNELS: usize = super::CHANNELS;

#[allow(unused)]
struct Signal<S, F> where S: num::Float {
	sample_rate: S,
	frames: Box<[F]>,
}

impl<S, F> Signal<S, F> where S: num::Float {
	fn len(&self) -> usize {
		self.frames.len()
	}
}

type StereoFrame = [f32; CHANNELS as usize];
type StereoSignal = Signal<f32, StereoFrame>;

#[derive(Clone)]
struct Tone<T, S> where T: num::Float, S: num::Float {
	pitch: T,
	duration: Seconds,
	amplitude: S,
}

impl<T, S> Tone<T, S> where T: num::Float, S: num::Float {
	fn new(letter_octave: LetterOctave, duration: Seconds, amplitude: S) -> Self {
		Tone { pitch: NumCast::from(letter_octave.hz()).unwrap(), duration, amplitude }
	}

	fn note_octave(note: Letter, octave: i32, duration: Seconds, amplitude: S) -> Self {
		Self::new(LetterOctave(note, octave), duration, amplitude)
	}
}

impl<T, S> Default for Tone<T, S> where T: num::Float, S: num::Float {
	fn default() -> Self {
		Tone::note_octave(Letter::C, 4, seconds(1.0), S::one())
	}
}

#[allow(unused)]
#[derive(Clone)]
enum Waveform<T, S> where
	T: num::Float, S: num::Float {
	Sin,
	Triangle(T),
	Harmonics(Box<[S]>, Box<[S]>),
	Square(T),
	Silence,
}

#[inline]
fn lerp_clip<T, S>(x0: T, x1: T, y0: S, y1: S, t: T) -> S
	where T: num::Float, S: num::Float {
	let v = NumCast::from((t - x0) / (x1 - x0)).unwrap();
	y0 + (y1 - y0) * S::zero().max(S::one().min(v))
}

impl<T, S> Waveform<T, S>
	where T: num::Float, S: num::Float {
	#[inline]
	fn sample(&self, phase: T) -> S where T: FloatConst {
		let phi = <S as NumCast>::from((phase + phase) * T::PI()).unwrap();
		match self {
			&Waveform::Sin => phi.sin(),
			&Waveform::Harmonics(ref hcos, ref hsin) => {
				let cos_comp =
					hcos.iter().enumerate()
						.fold(S::zero(), |sum, (i, f)| sum + *f * (phi * NumCast::from(i + 1).unwrap()).cos());
				let sin_comp =
					hsin.iter().enumerate()
						.fold(S::zero(), |sum, (i, f)| sum + *f * (phi * NumCast::from(i + 1).unwrap()).sin());
				cos_comp + sin_comp
			}
			&Waveform::Triangle(slant) => {
				if phase < slant {
					lerp_clip(T::zero(), slant, -S::one(), S::one(), phase)
				} else {
					lerp_clip(slant, T::one(), S::one(), -S::one(), phase)
				}
			}
			&Waveform::Square(duty_cycle) => {
				let s: T = (phase - duty_cycle).signum();
				NumCast::from(s).unwrap()
			}
			_ => S::zero(),
		}
	}
}

#[derive(Clone)]
struct Envelope<T, S>
	where T: num::Float, S: num::Float {
	attack: T,
	decay: T,
	sustain: S,
	release: T,
}

impl<T, S> Default for Envelope<T, S>
	where T: num::Float, S: num::Float {
	fn default() -> Self {
		Envelope {
			attack: T::zero(),
			decay: T::zero(),
			sustain: S::one(),
			release: T::zero(),
		}
	}
}

impl<T, S> Envelope<T, S>
	where T: num::Float, S: num::Float {
	#[allow(unused)]
	fn adsr(attack: T, decay: T, sustain: S, release: T) -> Self {
		Envelope {
			attack,
			decay,
			sustain,
			release,
		}
	}

	fn ramp_down(duration: Seconds) -> Self {
		Envelope {
			release: NumCast::from(duration.get()).unwrap(),
			..Default::default()
		}
	}

	fn gain(&self, duration: T, t: T) -> S {
		if t < self.attack {
			lerp_clip(T::zero(), self.attack, S::zero(), S::one(), t)
		} else if t < self.decay {
			lerp_clip(self.attack, self.attack + self.decay, S::one(), self.sustain, t)
		} else if t < duration - self.release {
			self.sustain
		} else if t < duration {
			lerp_clip(duration - self.release, duration, self.sustain, S::zero(), t)
		} else {
			S::zero()
		}
	}
}

#[derive(Clone)]
struct Oscillator<T, S>
	where T: num::Float, S: num::Float {
	waveform: Waveform<T, S>,
}

#[allow(unused)]
impl<T, S> Oscillator<T, S>
	where T: num::Float + 'static, S: num::Float + sample::Sample + 'static {
	fn sin() -> Self {
		Oscillator {
			waveform: Waveform::Sin,
		}
	}

	fn square() -> Self {
		Self::pwm(NumCast::from(0.5).unwrap())
	}

	fn pwm(duty_cycle: T) -> Self {
		Oscillator {
			waveform: Waveform::Square(duty_cycle),
		}
	}

	fn down_saw(amplitude: S) -> Self {
		Self::triangle(T::zero())
	}

	fn up_saw(amplitude: S) -> Self {
		Self::triangle(T::one())
	}

	fn triangle(slant: T) -> Self {
		Oscillator {
			waveform: Waveform::Triangle(slant),
		}
	}

	fn harmonics(hcos: &[S], hsin: &[S]) -> Self {
		Oscillator {
			waveform: Waveform::Harmonics(hcos.to_vec().into_boxed_slice(), hsin.to_vec().into_boxed_slice()),
		}
	}

	fn silence() -> Self {
		Oscillator {
			waveform: Waveform::Silence,
		}
	}

	#[inline]
	fn sample(&self, t: T) -> S where T: FloatConst {
		self.waveform.sample(t)
	}

	fn signal_function(self, tone: Tone<T, S>, envelope: Envelope<T, S>, pan: S) -> Box<Fn(T) -> [S; CHANNELS]>
		where T: FloatConst {
		let c_pan = [S::one() - pan, pan];
		let duration: T = NumCast::from(tone.duration.get()).unwrap();
		Box::new(move |t: T| {
			let t = NumCast::from(t).unwrap();
			let val = tone.amplitude * envelope.gain(duration, t) * self.sample((t * tone.pitch).fract());
			sample::Frame::from_fn(|channel| {
				(val * c_pan[channel]).to_sample()
			})
		})
	}
}

#[derive(Default, Clone)]
struct Voice {
	signal: Option<usize>,
	length: usize,
	position: usize,
}

impl Voice {
	fn new(signal_index: usize, length: usize) -> Self {
		Voice {
			signal: Some(signal_index),
			length,
			position: 0,
		}
	}

	fn remaining(&self) -> usize {
		self.length - self.position
	}

	fn advance(&mut self, l: usize) -> bool {
		self.position = usize::min(self.length, self.position + l);
		self.position >= self.length
	}
}

pub struct Multiplexer {
	#[allow(unused)]
	sample_rate: f64,
	wave_table: Vec<StereoSignal>,
	sample_map: HashMap<SoundEffect, usize>,
	voices: Vec<Voice>,
	playing_voice_index: BitSet,
	available_voice_index: Vec<usize>,
}

#[derive(Clone)]
pub struct Delay<S> where S: num::Float {
	time: Seconds,
	tail: Seconds,
	wet_dry: S,
	feedback: S,
}

impl<S> Delay<S> where S: num::Float {
	fn with_time(time: Seconds) -> Self {
		Delay::<S> {
			time,
			tail: time * 8.0,
			wet_dry: S::one(),
			feedback: NumCast::from(0.5).unwrap(),
		}
	}
}

impl<S> Default for Delay<S> where S: num::Float {
	fn default() -> Self { Delay::with_time(seconds(0.25)) }
}

#[derive(Clone)]
pub struct SignalBuilder<T, S>
	where T: num::Float, S: num::Float + sample::Sample {
	oscillator: Oscillator<T, S>,
	tone: Tone<T, S>,
	envelope: Envelope<T, S>,
	pan: S,
	sample_rate: T,
	delay: Delay<S>,
}

#[allow(unused)]
impl<T, S> SignalBuilder<T, S>
	where T: num::Float + 'static, S: num::Float + sample::Sample + 'static {
	fn new() -> Self {
		SignalBuilder {
			oscillator: Oscillator::sin(),
			tone: Tone::default(),
			envelope: Envelope::default(),
			sample_rate: NumCast::from(48000.0).unwrap(),
			pan: NumCast::from(0.5).unwrap(),
			delay: Delay::default(),
		}
	}

	fn from_oscillator(oscillator: Oscillator<T, S>) -> Self {
		SignalBuilder {
			oscillator,
			tone: Tone::default(),
			envelope: Envelope::ramp_down(seconds(1.0)),
			sample_rate: NumCast::from(48000.0).unwrap(),
			pan: NumCast::from(0.5).unwrap(),
			delay: Delay::default(),
		}
	}

	fn with_tone(&self, tone: Tone<T, S>) -> Self {
		SignalBuilder {
			tone,
			..self.clone()
		}
	}

	fn with_envelope(&self, envelope: Envelope<T, S>) -> Self {
		SignalBuilder {
			envelope,
			..self.clone()
		}
	}

	fn with_envelope_ramp_down(&self) -> Self {
		self.with_envelope(Envelope::ramp_down(self.tone.duration))
	}

	fn with_oscillator(&self, oscillator: Oscillator<T, S>) -> Self {
		SignalBuilder {
			oscillator,
			..self.clone()
		}
	}

	fn with_pan(&self, pan: S) -> Self {
		SignalBuilder {
			pan,
			..self.clone()
		}
	}

	fn with_delay(&self, delay: Delay<S>) -> Self {
		SignalBuilder {
			delay,
			..self.clone()
		}
	}

	fn with_delay_time(&self, time: Seconds) -> Self {
		self.with_delay(Delay {
			time,
			tail: time * 8.0f64,
			..self.delay.clone()
		})
	}

	fn build(&self) -> Signal<T, [S; CHANNELS]>
		where T: FloatConst {
		let f = self.oscillator.clone().signal_function(
			self.tone.clone(),
			self.envelope.clone(),
			self.pan);
		Signal::<T, [S; CHANNELS]>::new(self.sample_rate, self.tone.duration, f)
			.with_delay(self.delay.time, self.delay.tail, self.delay.wet_dry, self.delay.feedback)
	}

	fn render(&self, wave_table: &mut Vec<Signal<T, [S; CHANNELS]>>) -> usize
		where T: FloatConst {
		let signal = self.build();
		let index = wave_table.len();
		info!("Built signal[{}] with {} samples", index, signal.len());
		wave_table.push(signal);
		index
	}
}

impl Multiplexer {
	pub fn new(sample_rate: f64, max_voices: usize) -> Multiplexer {
		let mut wave_table = Vec::new();
		let mut sample_map = HashMap::new();
		{
			let mut map_effect = |effect: SoundEffect, wave_index: usize| {
				sample_map.insert(effect, wave_index);
				info!("Assigned {:?} to signal[{}]", effect, wave_index);
			};

			map_effect(SoundEffect::Startup, SignalBuilder::from_oscillator(
				Oscillator::harmonics(&[0., 0.1, 0., 0.2], &[0.6]))
				.with_tone(Tone::note_octave(Letter::A, 3, seconds(2.), 0.3))
				.with_envelope(Envelope::adsr(0.01, 0.5, 0.5, 0.5))
				.with_pan(0.25)
				.with_delay_time(seconds(1.0))
				.render(&mut wave_table));

			map_effect(SoundEffect::SelectMinion, SignalBuilder::from_oscillator(Oscillator::square())
				.with_tone(Tone::note_octave(Letter::G, 5, seconds(0.1), 0.1))
				.with_pan(0.8)
				.with_delay_time(seconds(0.05))
				.render(&mut wave_table));

			map_effect(SoundEffect::UserOption, SignalBuilder::from_oscillator(Oscillator::triangle(1.0))
				.with_tone(Tone::note_octave(Letter::C, 6, seconds(0.1), 0.1))
				.with_envelope(Envelope::adsr(0.01, 0.05, 0.9, 0.05))
				.with_pan(0.6)
				.with_delay_time(seconds(0.1))
				.render(&mut wave_table));

			map_effect(SoundEffect::Bullet(0), SignalBuilder::from_oscillator(Oscillator::triangle(0.75))
				.with_tone(Tone::note_octave(Letter::F, 6, seconds(0.05), 0.05))
				.with_envelope(Envelope::adsr(0., 0.01, 0.8, 0.))
				.with_pan(0.5)
				.with_delay_time(seconds(0.016))
				.render(&mut wave_table));

			map_effect(SoundEffect::GrowMinion, SignalBuilder::from_oscillator(Oscillator::sin())
				.with_tone(Tone::note_octave(Letter::C, 3, seconds(0.05), 0.05))
				.with_envelope(Envelope::adsr(0., 0.01, 0.8, 0.))
				.with_pan(0.5)
				.with_delay_time(seconds(0.05))
				.render(&mut wave_table));

			map_effect(SoundEffect::Fertilised, SignalBuilder::from_oscillator(Oscillator::sin())
				.with_tone(Tone::note_octave(Letter::C, 4, seconds(0.3), 0.1))
				.with_pan(0.6)
				.with_delay_time(seconds(0.25))
				.render(&mut wave_table));

			map_effect(SoundEffect::NewSpore, SignalBuilder::from_oscillator(
				Oscillator::harmonics(&[0., 0.3, 0., 0.1], &[0.6]))
				.with_tone(Tone::note_octave(Letter::F, 5, seconds(0.3), 0.1))
				.with_pan(0.3)
				.with_delay_time(seconds(0.33))
				.render(&mut wave_table));

			map_effect(SoundEffect::NewMinion, SignalBuilder::from_oscillator(Oscillator::sin())
				.with_tone(Tone::note_octave(Letter::A, 4, seconds(0.5), 0.1))
				.with_pan(0.55).with_delay_time(seconds(0.25))
				.render(&mut wave_table));

			map_effect(SoundEffect::DieMinion, SignalBuilder::from_oscillator(Oscillator::sin())
				.with_tone(Tone::note_octave(Letter::Eb, 3, seconds(0.5), 0.2))
				.with_pan(1.)
				.with_delay_time(seconds(0.3))
				.render(&mut wave_table));
		}

		let voices = vec![Voice::default(); max_voices];
		let playing_voice_index = BitSet::with_capacity(max_voices);
		let available_voice_index = (0..max_voices).rev().collect();

		Multiplexer {
			sample_rate,
			wave_table,
			sample_map,
			voices,
			playing_voice_index,
			available_voice_index,
		}
	}

	fn free_voice(&mut self, voice_index: usize) {
		self.voices[voice_index].signal = None;
		self.playing_voice_index.remove(voice_index);
		self.available_voice_index.push(voice_index);
	}

	fn allocate_voice(&mut self, voice: Voice) -> Option<usize> {
		let allocated = self.available_voice_index.pop();
		if let Some(voice_index) = allocated {
			self.playing_voice_index.insert(voice_index);
			self.voices[voice_index] = voice;
		}
		allocated
	}

	pub fn audio_requested(&mut self, buffer: &mut [StereoFrame]) {
		sample::slice::equilibrium(buffer);
		let mut terminated_voices = BitSet::with_capacity(self.voices.len());
		for voice_index in &self.playing_voice_index {
			let voice = self.voices[voice_index].clone();
			if let Some(signal_index) = voice.signal {
				let frames = &self.wave_table[signal_index].frames[voice.position..];
				let len = buffer.len().min(voice.remaining());
				// TODO: how do we unroll this?
				for channel in 0..CHANNELS {
					for idx in 0..len {
						buffer[idx][channel] += frames[idx][channel];
					}
				}

				if self.voices[voice_index].advance(len) {
					// returns true on EOF
					terminated_voices.insert(voice_index);
				}
				else {
					trace!("Voice {} playing", voice_index);
				}
			}
		}
		for voice_index in &terminated_voices {
			self.free_voice(voice_index);
			trace!("Voice {} stopped", voice_index);
		}
	}

	pub fn trigger(&mut self, effect: SoundEffect) {
		if let Some(signal_index) = self.sample_map.get(&effect).map(|t| *t) {
			let signal_length = self.wave_table[signal_index].len();
			if let Some(index) = self.allocate_voice(Voice::new(signal_index, signal_length)) {
				trace!("Voice {} playing, {:?}", index, effect);
			}
			else {
				warn!("Not enough voices, skipped {:?}", effect);
			}
		}
	}
}

#[allow(unused)]
impl<S, F> Signal<S, F> where S: num::Float {
	fn new<V>(sample_rate: S, duration: Seconds, f: Box<V>) -> Signal<S, F>
		where V: Fn(S) -> F + ? Sized {
		let samples: usize = (duration.get() * sample_rate.to_f64().unwrap()).round() as usize;
		let frames = (0..samples)
			.map(|i| S::from(i).unwrap() / sample_rate)
			.map(|t| f(t)).collect::<Vec<F>>();
		Signal {
			sample_rate,
			frames: frames.into_boxed_slice(),
		}
	}

	fn duration(&self) -> Seconds {
		seconds(self.sample_rate.to_f64().unwrap() * self.frames.len() as f64)
	}

	fn sample_rate(&self) -> S {
		self.sample_rate
	}
}

impl<S, T> Signal<S, [T; CHANNELS]>
	where S: num::Float, T: num::Float + sample::Sample {
	fn with_delay(self, time: Seconds, tail: Seconds, wet_dry: T, feedback: T) -> Self {
		use sample::Signal;
		let wet_ratio = wet_dry;
		let dry_ratio: T = T::one() - wet_dry;
		let source_length = self.frames.len();
		let sample_rate = self.sample_rate.to_f64().unwrap();
		let delay_length = (time.get() * sample_rate).round() as usize;
		let tail_length = (tail.get() * sample_rate).round() as usize;
		let dest_length = source_length + tail_length;
		let mut delay_buffer: Vec<[T; CHANNELS]> = sample::signal::equilibrium().take(delay_length).collect();
		let mut dest_buffer: Vec<[T; CHANNELS]> = Vec::with_capacity(source_length + tail_length);

		for i in 0..dest_length {
			let tram_ptr = i % delay_length;
			let src = *self.frames.get(i).unwrap_or(&[T::zero(), T::zero()]);
			let delay_effect = delay_buffer[tram_ptr];
			let wet: [T; CHANNELS] = sample::Frame::from_fn(move |channel| { wet_ratio * src[channel] + delay_effect[channel] });
			dest_buffer.push(sample::Frame::from_fn(move |channel| { dry_ratio * src[channel] + wet[channel] }));
			delay_buffer[tram_ptr] = sample::Frame::from_fn(move |channel| { feedback * wet[CHANNELS - 1 - channel] }); // ping-pong
		}
		self::Signal {
			sample_rate: self.sample_rate,
			frames: dest_buffer.into_boxed_slice(),
		}
	}
}
