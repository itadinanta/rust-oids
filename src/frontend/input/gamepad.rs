use gilrs;
use gilrs::Gilrs;
use gilrs::ev::filter::{Filter, Repeat};

use frontend::input;
use frontend::input::EventMapper;

pub struct GamepadEventLoop {
	repeat_filter: Repeat,
	gilrs: Gilrs,
}

impl input::EventMapper<gilrs::Event> for GamepadEventLoop {
	fn translate(&self, _e: &gilrs::Event) -> Option<input::Event> {
		None
	}
}

impl GamepadEventLoop {
	pub fn new() -> Self {
		GamepadEventLoop {
			repeat_filter: Repeat::new(),
			gilrs: Gilrs::new(),
		}
	}

	pub fn poll_events<F>(&mut self, mut on_input_event: F)
		where F: FnMut(input::Event) {
		let repeat_filter = self.repeat_filter.clone();
		while let Some(ev) = Filter::filter(&self.gilrs.next_event(), &repeat_filter, &self.gilrs) {
			self.gilrs.update(&ev);
			println!("{:?}", ev);
			self.translate(&ev).map(&mut on_input_event);
		};
		self.gilrs.inc();
	}
}
