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
	fn translate(&self, _e: &gilrs::Event) -> Option<input::Event> {
		None
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
