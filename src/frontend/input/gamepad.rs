use gilrs;
use gilrs::Gilrs;
use std::collections::HashMap;

use frontend::input;
use frontend::input::EventMapper;

pub struct GamepadEventLoop {
	//	repeat_filter: Repeat,
	gilrs: Gilrs,
	gamepad_ids: HashMap<gilrs::GamepadId, usize>,
}

impl input::EventMapper<gilrs::Event> for GamepadEventLoop {
	fn translate(&self, e: &gilrs::Event) -> Option<input::Event> {
		#[inline]
		fn to_key(button: gilrs::Button) -> Option<input::Key> {
			use frontend::input::Key::*;
			match button {
				gilrs::Button::South => Some(GamepadSouth),
				gilrs::Button::East => Some(GamepadEast),
				gilrs::Button::North => Some(GamepadNorth),
				gilrs::Button::West => Some(GamepadWest),
				// Triggers
				gilrs::Button::LeftTrigger => Some(GamepadL1),
				gilrs::Button::RightTrigger => Some(GamepadR1),
				gilrs::Button::LeftTrigger2 => Some(GamepadL2),
				gilrs::Button::RightTrigger2 => Some(GamepadR2),
				// Menu Pad
				gilrs::Button::Select => Some(GamepadSelect),
				gilrs::Button::Start => Some(GamepadStart),
				// Sticks
				gilrs::Button::LeftThumb => Some(GamepadL3),
				gilrs::Button::RightThumb => Some(GamepadR3),
				// D-Pad
				gilrs::Button::DPadDown => Some(GamepadDPadDown),
				gilrs::Button::DPadUp => Some(GamepadDPadUp),
				gilrs::Button::DPadLeft => Some(GamepadDPadLeft),
				gilrs::Button::DPadRight => Some(GamepadDPadRight),

				_ => None,
			}
		}

		fn from_axis(axis: gilrs::Axis) -> Option<input::Axis> {
			use frontend::input::Axis::*;
			match axis {
				gilrs::Axis::LeftStickX => Some(LStickX),
				gilrs::Axis::LeftStickY => Some(LStickY),
				gilrs::Axis::RightStickX => Some(RStickX),
				gilrs::Axis::RightStickY => Some(RStickY),
				_ => None,
			}
		}

		fn from_button(axis: gilrs::Button) -> Option<input::Axis> {
			use frontend::input::Axis::*;
			match axis {
				gilrs::Button::LeftTrigger2 => Some(L2),
				gilrs::Button::RightTrigger2 => Some(R2),
				_ => None,
			}
		}

		match e.event {
			gilrs::EventType::ButtonPressed(button, _) =>
				to_key(button).map(|key| input::Event::GamepadButton(self.map_id(e.id), input::State::Down, key)),
			gilrs::EventType::ButtonReleased(button, _) =>
				to_key(button).map(|key| input::Event::GamepadButton(self.map_id(e.id), input::State::Up, key)),
			gilrs::EventType::ButtonChanged(gilrs::Button::RightTrigger2, value, _) =>
				from_button(gilrs::Button::RightTrigger2)
					.map(|axis| input::Event::GamepadAxis(self.map_id(e.id), value, axis)),
			gilrs::EventType::ButtonChanged(gilrs::Button::LeftTrigger2, value, _) =>
				from_button(gilrs::Button::LeftTrigger2)
					.map(|axis| input::Event::GamepadAxis(self.map_id(e.id), value, axis)),
			gilrs::EventType::AxisChanged(axis, value, _) =>
				from_axis(axis).map(|axis| input::Event::GamepadAxis(self.map_id(e.id), value, axis)),
			_ => None,
		}
	}
}

impl GamepadEventLoop {
	pub fn new() -> Option<Self> {
		let result = Gilrs::new();
		let mut gamepad_ids = HashMap::new();

		match result {
			Err(err) => {
				println!("Error initializing gamepad: {:?}", err);
				None
			}
			Ok(gilrs) => {
				for (_id, gamepad) in gilrs.gamepads() {
					info!("{} [{:?}] is {:?}", gamepad.name(), gamepad.id(), gamepad.power_info());
					let next_index = gamepad_ids.len();
					gamepad_ids.insert(gamepad.id(), next_index);
				}
				Some(GamepadEventLoop { gilrs, gamepad_ids })
			}
		}
	}

	pub fn map_id(&self, gilrs_id: gilrs::GamepadId) -> usize {
		self.gamepad_ids.get(&gilrs_id).copied().unwrap_or_default()
	}

	pub fn poll_events<F>(&mut self, mut on_input_event: F)
	where F: FnMut(input::Event) {
		while let Some(ev) = self.gilrs.next_event() {
			self.gilrs.update(&ev);
			trace!("{:?}", ev);
			self.translate(&ev).map(&mut on_input_event);
		}
		self.gilrs.inc();
	}
}
