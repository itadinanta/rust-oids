use gilrs;
use gilrs::Gilrs;
// use gilrs::ev::filter::{Filter, Repeat};

use frontend::input;
use frontend::input::EventMapper;

pub struct GamepadEventLoop {
	//	repeat_filter: Repeat,
	gilrs: Gilrs,
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
				gilrs::Button::DPadUp => Some(GamepadDPadUp),
				gilrs::Button::DPadDown => Some(GamepadDPadDown),
				gilrs::Button::DPadLeft => Some(GamepadDPadLeft),
				gilrs::Button::DPadRight => Some(GamepadDPadRight),

				_ => None
			}
		}

		fn to_axis(axis: gilrs::Axis) -> Option<input::Axis> {
			use frontend::input::Axis::*;
			match axis {
				gilrs::Axis::LeftStickX => Some(LStickX),
				gilrs::Axis::LeftStickY => Some(LStickY),
				gilrs::Axis::RightStickX => Some(RStickX),
				gilrs::Axis::RightStickY => Some(RStickY),
				gilrs::Axis::LeftTrigger2 => Some(L2),
				gilrs::Axis::RightTrigger2 => Some(R2),
				_ => None
			}
		}

		match e.event {
			gilrs::EventType::ButtonPressed(button, _) =>
				to_key(button).map(|key| input::Event::GamepadButton(e.id, input::State::Down, key)),
			gilrs::EventType::ButtonReleased(button, _) =>
				to_key(button).map(|key| input::Event::GamepadButton(e.id, input::State::Up, key)),
			gilrs::EventType::AxisChanged(gilrs::Axis::RightTrigger2, value, _) =>
				to_axis(gilrs::Axis::RightTrigger2)
					.map(|axis| input::Event::GamepadAxis(e.id, if cfg!(linux) { 0.5 - 0.5 * value } else { value }, axis)),
			gilrs::EventType::AxisChanged(gilrs::Axis::LeftTrigger2, value, _) =>
				to_axis(gilrs::Axis::LeftTrigger2)
					.map(|axis| input::Event::GamepadAxis(e.id, if cfg!(linux) { 0.5 + 0.5 * value } else { value }, axis)),
			gilrs::EventType::AxisChanged(axis, value, _) =>
				to_axis(axis).map(|axis| input::Event::GamepadAxis(e.id, value, axis)),
			_ => None
		}
	}
}

impl GamepadEventLoop {
	pub fn new() -> Self {
		let gilrs = Gilrs::new();
		for (_id, gamepad) in gilrs.gamepads() {
			info!("{} is {:?}", gamepad.name(), gamepad.power_info());
		}
		GamepadEventLoop {
			gilrs,
		}
	}

	pub fn poll_events<F>(&mut self, mut on_input_event: F)
		where F: FnMut(input::Event) {
		while let Some(ev) = self.gilrs.next_event() {
			self.gilrs.update(&ev);
			trace!("{:?}", ev);
			self.translate(&ev).map(&mut on_input_event);
		};
		self.gilrs.inc();
	}
}
