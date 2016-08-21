//! Input state, including current mouse position and button click
//! TODO: add keyboard presses
use cgmath::Vector2;
use bit_set::BitSet;

pub struct InputState {
	key_pressed: BitSet,
	key_ack: BitSet,
	mouse_position: Vector2<f32>,
}

impl Default for InputState {
	fn default() -> Self {
		InputState {
			key_pressed: BitSet::new(),
			key_ack: BitSet::new(),
			mouse_position: Vector2::new(0., 0.),
		}
	}
}

#[derive(Copy,Clone)]
pub enum State {
	Down,
	Up,
}

#[derive(Copy,Clone)]
pub enum Key {
	A,
	B,
	C,
	D,
	E,
	F,
	G,
	H,
	I,
	J,
	K,
	L,
	M,
	N,
	O,
	P,
	Q,
	R,
	S,
	T,
	U,
	V,
	W,
	X,
	Y,
	Z,
	F1,
	F2,
	F3,
	F4,
	F5,
	F6,
	F7,
	F8,
	F9,
	F10,
	F11,
	F12,
	N0,
	N1,
	N2,
	N3,
	N4,
	N5,
	N6,
	N7,
	N8,
	N9,
	Plus,
	Minus,
	Backspace,

	Up,
	Down,
	Left,
	Right,

	Del,
	Ins,
	Home,
	End,
	Enter,
	PageUp,
	PageDown,

	Kp1,
	Kp2,
	Kp3,
	Kp4,
	Kp5,
	Kp6,
	Kp7,
	Kp8,
	Kp9,
	Kp0,
	KpPlus,
	KpMinus,
	KpDel,
	KpIns,
	KpHome,
	KpEnd,
	KpEnter,
	KpPageUp,
	KpPageDown,

	LShift,
	RShift,
	LAlt,
	RAlt,
	LSuper,
	RSuper,
	LCtrl,
	RCtrl,
	CapsLock,

	Space,
	Esc,
	Tab,
	PrintScreen,
	SysRq,

	MouseLeft,
	MouseRight,
	MouseMiddle,
	MouseScrollUp,
	MouseScrollDown,
}

pub enum Event {
	Key(State, Key),
	Mouse(f32, f32),
}


impl InputState {
	pub fn event(&mut self, event: &Event) {
		match event {
			&Event::Key(state, key) => self.key(state, key),
			&Event::Mouse(x, y) => self.mouse_at(Vector2::new(x, y)),
		}
	}

	pub fn key_pressed(&self, b: Key) -> bool {
		self.key_pressed.contains(b as usize)
	}

	pub fn any_ctrl_pressed(&self) -> bool {
		self.any_key_pressed(&[Key::LCtrl, Key::RCtrl])
	}

	pub fn any_alt_pressed(&self) -> bool {
		self.any_key_pressed(&[Key::LAlt, Key::RAlt])
	}

	pub fn any_super_pressed(&self) -> bool {
		self.any_key_pressed(&[Key::LSuper, Key::RSuper])
	}

	pub fn any_key_pressed(&self, b: &[Key]) -> bool {
		let other: BitSet = b.into_iter().map(|k| *k as usize).collect();
		!self.key_pressed.is_disjoint(&other)
	}

	pub fn chord_pressed(&self, b: &[Key]) -> bool {
		let other: BitSet = b.into_iter().map(|k| *k as usize).collect();
		self.key_pressed.is_superset(&other)

	}

	pub fn key_once(&mut self, b: Key) -> bool {
		if self.key_ack.contains(b as usize) {
			false
		} else {
			self.key_ack.insert(b as usize);
			self.key_pressed.contains(b as usize)
		}
	}

	pub fn mouse_position(&self) -> Vector2<f32> {
		self.mouse_position
	}

	fn key(&mut self, state: State, b: Key) {
		self.key_ack.remove(b as usize);
		match state {
			State::Down => self.key_pressed.insert(b as usize),
			State::Up => self.key_pressed.remove(b as usize),
		};
	}

	pub fn mouse_at(&mut self, pos: Vector2<f32>) {
		self.mouse_position = pos;
	}
}

pub trait EventMapper<T, S> {
	fn eventmap(src: &T) -> Event;
}
