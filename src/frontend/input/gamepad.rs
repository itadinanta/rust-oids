use gilrs;
use gilrs::Gilrs;
// use gilrs::ev::filter::{Filter, Repeat};

use frontend::input;
use frontend::input::EventMapper ;

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
				gilrs::Axis::LeftTrigger => Some(L2),
				gilrs::Axis::RightTrigger => Some(R2),
				_ => None
			}
		}

		match e.event {
			gilrs::EventType::ButtonPressed(button, _) =>
				to_key(button).map(|key| input::Event::GamepadButton(e.id, input::State::Down, key)),
			gilrs::EventType::ButtonReleased(button, _) =>
				to_key(button).map(|key| input::Event::GamepadButton(e.id, input::State::Up, key)),
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
			println!("{} is {:?}", gamepad.name(), gamepad.power_info());
		}
		GamepadEventLoop {
//			repeat_filter: Repeat::new(),
			gilrs,
		}
	}

	pub fn poll_events<F>(&mut self, mut on_input_event: F)
		where F: FnMut(input::Event) {
		//let repeat_filter = self.repeat_filter.clone();
		//while let Some(ev) = Filter::filter(&self.gilrs.next_event(), &repeat_filter, &self.gilrs) {
		while let Some(ev) = self.gilrs.next_event() {
			self.gilrs.update(&ev);
			println!("{:?}", ev);
			self.translate(&ev).map(&mut on_input_event);
		};
		self.gilrs.inc();
	}
}
