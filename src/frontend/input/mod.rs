//! Input state, including current mouse position and button click
mod gamepad;

use core::geometry;
use core::util::History;
use core::geometry::Position;
use bit_set::BitSet;
use std::iter::Iterator;

#[derive(Clone)]
enum DragState {
	Nothing,
	Hold(Key, Position),
}

pub enum Dragging {
	Nothing,
	Begin(Key, Position),
	Dragging(Key, Position, Position),
	End(Key, Position, Position, Position),
}

#[derive(Default, Clone)]
pub struct GamepadState {
	connected: bool,
	button_pressed: BitSet,
	button_ack: BitSet,
	x: f32,
	y: f32,
}

const MAX_GAMEPADS: usize = 2;

pub struct InputState {
	gamepad: Vec<GamepadState>,
	key_pressed: BitSet,
	key_ack: BitSet,
	drag_state: DragState,
	mouse_history: History<Position>,
	mouse_position: Position,
}

impl Default for InputState {
	fn default() -> Self {
		InputState {
			gamepad: vec![GamepadState::default(); MAX_GAMEPADS],
			key_pressed: BitSet::new(),
			key_ack: BitSet::new(),
			drag_state: DragState::Nothing,
			mouse_history: History::new(60),
			mouse_position: geometry::origin(),
		}
	}
}

#[derive(Copy, Clone)]
pub enum State {
	Down,
	Up,
}

#[allow(dead_code)]
#[derive(Copy, Clone, Eq, PartialEq)]
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

	Backtick,
	OpenBracket,
	CloseBracket,
	Semicolon,
	Apostrophe,
	Tilde,

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

	MouseLeft,
	MouseRight,
	MouseMiddle,
	MouseScrollUp,
	MouseScrollDown,
}

pub enum Event {
	Key(State, Key),
	Mouse(Position),
	GamepadPoll(usize),
}

#[allow(dead_code)]
impl InputState {
	pub fn event(&mut self, event: &Event) {
		match event {
			&Event::Key(state, key) => self.key(state, key),
			&Event::Mouse(position) => self.mouse_at(position),
			&Event::GamepadPoll(id) => self.gamepad_poll(id),
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

	pub fn gamepad_poll(&mut self, id: usize) {}

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

	pub fn mouse_position(&self) -> Position {
		self.mouse_position
	}

	fn key(&mut self, state: State, b: Key) {
		self.key_ack.remove(b as usize);
		match state {
			State::Down => self.key_pressed.insert(b as usize),
			State::Up => self.key_pressed.remove(b as usize),
		};
	}

	pub fn mouse_at(&mut self, pos: Position) {
		self.mouse_position = pos;
	}

	pub fn dragging(&mut self, key: Key, pos: Position) -> Dragging {
		let (drag_state, displacement) = match &self.drag_state {
			&DragState::Nothing => {
				if self.key_pressed(key) {
					self.mouse_history.clear();
					(DragState::Hold(key, pos), Dragging::Begin(key, pos))
				} else {
					(DragState::Nothing, Dragging::Nothing)
				}
			}
			&DragState::Hold(held, start) if held == key => {
				let hold = if self.key_pressed(key) {
					(DragState::Hold(key, start), Dragging::Dragging(key, start, pos))
				} else {
					let prev = self.mouse_history.into_iter().next().unwrap_or(self.mouse_position);
					(DragState::Nothing, Dragging::End(key, start, pos, prev))
				};
				self.mouse_history.push(self.mouse_position);
				hold
			}
			_ => (self.drag_state.clone(), Dragging::Nothing),
		};
		self.drag_state = drag_state;
		displacement
	}
}

pub trait EventMapper<T> {
	fn translate(&self, e: &T) -> Option<Event>;
}
