//! Input state, including current mouse position and button click
pub mod gamepad;

pub use self::gamepad::GamepadEventLoop;

use core::geometry;
use core::util::History;
use core::geometry::Position;
use core::view::ViewTransform;
use bit_set::BitSet;
use std::iter::Iterator;
use std::collections::HashMap;

#[derive(Clone)]
enum DragState {
	Nothing,
	Hold(Key, Position),
}

#[derive(Clone)]
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
	pub button_pressed_last: BitSet,
	pub axis: [AxisValue; MAX_AXIS],
}

pub struct InputState {
	gamepad: HashMap<usize, GamepadState>,
	key_pressed: BitSet,
	key_pressed_last: BitSet,
	drag_state: DragState,
	dragging: Dragging,
	mouse_history: History<Position>,
	mouse_position: Position,
}

impl Default for GamepadState {
	fn default() -> Self {
		GamepadState {
			connected: false,
			button_pressed: BitSet::new(),
			button_pressed_last: BitSet::new(),
			axis: [0.0; MAX_AXIS],
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
			key_pressed_last: BitSet::new(),
			drag_state: DragState::Nothing,
			dragging: Dragging::Nothing,
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
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
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
		match state {
			State::Down => self.button_pressed.insert(b as usize),
			State::Up => self.button_pressed.remove(b as usize),
		};
	}

	pub fn button_pressed(&self, b: Key) -> bool {
		self.button_pressed.contains(b as usize)
	}

	pub fn button_once(&self, b: Key) -> bool {
		self.button_pressed.contains(b as usize) &&
			!self.button_pressed_last.contains(b as usize)
	}

	pub fn update_button_pressed(&mut self) {
		self.button_pressed_last = self.button_pressed.clone();
	}

	fn axis(&mut self, value: AxisValue, axis: Axis) {
		self.axis[axis as usize] = value;
	}
}

pub trait InputRead {
	fn key_pressed(&self, b: Key) -> bool;
	fn key_once(&self, b: Key) -> bool;
	fn any_key_pressed(&self, b: &[Key]) -> bool;
	fn any_ctrl_pressed(&self) -> bool;
	fn any_alt_pressed(&self) -> bool;
	fn any_super_pressed(&self) -> bool;
	fn chord_pressed(&self, b: &[Key]) -> bool;
	fn gamepad_button_pressed(&self, gamepad_id: usize, b: Key) -> bool;
	fn gamepad_axis(&self, gamepad_id: usize, axis: Axis) -> AxisValue;
	fn gamepad_button_once(&self, gamepad_id: usize, b: Key) -> bool;
	fn mouse_position(&self) -> Position;
	fn dragging(&self) -> Dragging;
}

#[allow(dead_code)]
impl InputRead for InputState {
	fn key_pressed(&self, b: Key) -> bool {
		self.key_pressed.contains(b as usize)
	}

	fn key_once(&self, b: Key) -> bool {
		self.key_pressed.contains(b as usize) &&
			!self.key_pressed_last.contains(b as usize)
	}

	fn any_key_pressed(&self, b: &[Key]) -> bool {
		let other: BitSet = b.iter().map(|k| *k as usize).collect();
		!self.key_pressed.is_disjoint(&other)
	}

	fn any_ctrl_pressed(&self) -> bool {
		self.any_key_pressed(&[Key::LCtrl, Key::RCtrl])
	}

	fn any_alt_pressed(&self) -> bool {
		self.any_key_pressed(&[Key::LAlt, Key::RAlt])
	}

	fn any_super_pressed(&self) -> bool {
		self.any_key_pressed(&[Key::LSuper, Key::RSuper])
	}

	fn chord_pressed(&self, b: &[Key]) -> bool {
		let other: BitSet = b.iter().map(|k| *k as usize).collect();
		self.key_pressed.is_superset(&other)
	}

	fn gamepad_button_pressed(&self, gamepad_id: usize, b: Key) -> bool {
		self.gamepad.get(&gamepad_id)
			.map(|gamepad| gamepad.button_pressed(b))
			.unwrap_or_default()
	}

	fn gamepad_axis(&self, gamepad_id: usize, axis: Axis) -> AxisValue {
		self.gamepad(gamepad_id)
			.or_else(|| self.gamepad(0))
			.map(|state| state.axis[axis as usize])
			.unwrap_or_default()
	}

	fn gamepad_button_once(&self, gamepad_id: usize, b: Key) -> bool {
		self.gamepad.get(&gamepad_id)
			.map(|gamepad| gamepad.button_once(b))
			.unwrap_or_default()
	}

	fn mouse_position(&self) -> Position {
		self.mouse_position
	}

	fn dragging(&self) -> Dragging {
		self.dragging.clone()
	}
}

#[allow(dead_code)]
impl InputState {
	pub fn event(&mut self, event: &Event) {
		match *event {
			Event::Key(state, key) => self.key(state, key),
			Event::Mouse(position) => self.mouse_at(position),
			Event::GamepadButton(id, state, button) => self.gamepad_button(id, state, button),
			Event::GamepadAxis(id, axis, position) => self.gamepad_axis_update(id, axis, position),
		}
	}

	pub fn pre_update<V>(&mut self, view_transform: &V) where V: ViewTransform {
		let mouse_window_pos = self.mouse_position();
		let mouse_view_pos = view_transform.to_view(mouse_window_pos);
		// TODO: generalise, for any button. Only RMB is supported otherwise
		self.update_dragging(Key::MouseRight, mouse_view_pos);
	}

	pub fn post_update(&mut self) {
		self.update_mouse_scroll();
		self.update_key_pressed();
		self.update_gamepad_button_pressed();
	}

	fn gamepad(&self, gamepad_id: usize) -> Option<&GamepadState> {
		self.gamepad.get(&gamepad_id)
	}

	fn mouse_at(&mut self, pos: Position) {
		self.mouse_position = pos;
	}

	fn key(&mut self, state: State, b: Key) {
		match state {
			State::Down => self.key_pressed.insert(b as usize),
			State::Up => self.key_pressed.remove(b as usize),
		};
	}

	fn gamepad_mut(&mut self, gamepad_id: usize) -> &mut GamepadState {
		self.gamepad
			.entry(gamepad_id)
			.or_insert_with(GamepadState::default)
	}

	fn gamepad_button(&mut self, gamepad_id: usize, state: State, button: Key) {
		self.key(state, button);
		self.gamepad_mut(gamepad_id).button(state, button);
	}

	fn gamepad_axis_update(&mut self, gamepad_id: usize, value: AxisValue, axis: Axis) {
		self.gamepad_mut(gamepad_id).axis(value, axis);
	}

	fn update_key_pressed(&mut self) {
		self.key_pressed_last = self.key_pressed.clone();
	}

	fn update_mouse_scroll(&mut self) {
		// Scroll events don't release keys, ever
		self.key(State::Up, Key::MouseScrollUp);
		self.key(State::Up, Key::MouseScrollDown);
	}

	fn update_gamepad_button_pressed(&mut self) {
		for gamepad in self.gamepad.values_mut() {
			gamepad.update_button_pressed();
		}
	}

	fn update_dragging(&mut self, key: Key, pos: Position) {
		let (drag_state, displacement) = match self.drag_state {
			DragState::Nothing => {
				if self.key_pressed(key) {
					self.mouse_history.clear();
					(DragState::Hold(key, pos), Dragging::Begin(key, pos))
				} else {
					(DragState::Nothing, Dragging::Nothing)
				}
			}
			DragState::Hold(held, start) if held == key => {
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
		self.dragging = displacement;
	}
}

pub trait EventMapper<T> {
	fn translate(&self, e: &T) -> Option<Event>;
}
