//! Input state, including current mouse position and button click
pub mod gamepad;

pub use self::gamepad::GamepadEventLoop;

use core::geometry;
use core::util::History;
use core::geometry::Position;
use bit_set::BitSet;
use std::iter::Iterator;
use std::collections::HashMap;

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

const MAX_AXIS: usize = 6;

#[derive(Clone)]
pub struct GamepadState {
	pub connected: bool,
	pub button_pressed: BitSet,
	pub button_ack: BitSet,
	pub axis: [AxisValue; MAX_AXIS],
}

pub struct InputState {
	gamepad: HashMap<usize, GamepadState>,
	key_pressed: BitSet,
	key_ack: BitSet,
	drag_state: DragState,
	mouse_history: History<Position>,
	mouse_position: Position,
}

impl Default for GamepadState {
	fn default() -> Self {
		GamepadState {
			connected: false,
			button_pressed: BitSet::new(),
			button_ack: BitSet::new(),
			axis: [0., 0., 0., 0., 1., 1.], // for some reason R button 1 is notpressed
		}
	}
}

impl Default for InputState {
	fn default() -> Self {
		let mut default_map = HashMap::new();
		default_map.insert(0usize, GamepadState::default());
		InputState {
			gamepad: default_map,
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

	GamepadEast,
	GamepadNorth,
	GamepadWest,
	GamepadSouth,
	GamepadDPadRight,
	GamepadDPadUp,
	GamepadDPadLeft,
	GamepadDPadDown,
	GamepadSelect,
	GamepadStart,
	GamepadR1,
	GamepadL1,
	GamepadR2,
	GamepadL2,
	GamepadR3,
	GamepadL3,

	GamepadLStickLeft,
	GamepadLStickUp,
	GamepadLStickRight,
	GamepadLStickDown,

	GamepadRStickLeft,
	GamepadRStickUp,
	GamepadRStickRight,
	GamepadRStickDown,
}

#[allow(unused)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Axis {
	LStickX,
	LStickY,
	RStickX,
	RStickY,
	L2,
	R2,
}

pub type AxisValue = f32;

pub enum Event {
	Key(State, Key),
	Mouse(Position),
	GamepadButton(usize, State, Key),
	GamepadAxis(usize, AxisValue, Axis),
}

#[allow(dead_code)]
impl GamepadState {
	fn button(&mut self, state: State, b: Key) {
		self.button_ack.remove(b as usize);
		match state {
			State::Down => self.button_pressed.insert(b as usize),
			State::Up => self.button_pressed.remove(b as usize),
		};
	}

	pub fn button_pressed(&self, b: Key) -> bool {
		self.button_pressed.contains(b as usize)
	}

	pub fn button_once(&mut self, b: Key) -> bool {
		if self.button_ack.contains(b as usize) {
			false
		} else {
			self.button_ack.insert(b as usize);
			self.button_pressed.contains(b as usize)
		}
	}

	fn axis(&mut self, value: AxisValue, axis: Axis) {
		self.axis[axis as usize] = value;
	}
}

#[allow(dead_code)]
impl InputState {
	pub fn event(&mut self, event: &Event) {
		match event {
			&Event::Key(state, key) => self.key(state, key),
			&Event::Mouse(position) => self.mouse_at(position),
			&Event::GamepadButton(id, state, button) => self.gamepad_button(id, state, button),
			&Event::GamepadAxis(id, axis, position) => self.gamepad_axis_update(id, axis, position),
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

	fn gamepad(&self, gamepad_id: usize) -> Option<&GamepadState> {
		self.gamepad.get(&gamepad_id)
	}

	fn gamepad_mut(&mut self, gamepad_id: usize) -> &mut GamepadState {
		self.gamepad
			.entry(gamepad_id)
			.or_insert(GamepadState::default())
	}

	fn gamepad_button(&mut self, gamepad_id: usize, state: State, button: Key) {
		self.key(state, button);
		self.gamepad_mut(gamepad_id).button(state, button);
	}

	fn gamepad_axis_update(&mut self, gamepad_id: usize, value: AxisValue, axis: Axis) {
		self.gamepad_mut(gamepad_id).axis(value, axis);
	}

	pub fn gamepad_axis(&self, gamepad_id: usize, axis: Axis) -> AxisValue {
		self.gamepad(gamepad_id)
			.or_else(|| self.gamepad(0))
			.map(|state| state.axis[axis as usize])
			.unwrap_or_default()
	}

	pub fn any_key_pressed(&self, b: &[Key]) -> bool {
		let other: BitSet = b.into_iter().map(|k| *k as usize).collect();
		!self.key_pressed.is_disjoint(&other)
	}

	pub fn chord_pressed(&self, b: &[Key]) -> bool {
		let other: BitSet = b.into_iter().map(|k| *k as usize).collect();
		self.key_pressed.is_superset(&other)
	}

	pub fn gamepad_button_once(&mut self, gamepad_id: usize, b: Key) -> bool {
		self.gamepad.get_mut(&gamepad_id)
			.map(|gamepad| gamepad.button_once(b))
			.unwrap_or_default()
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
