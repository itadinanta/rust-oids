use backend::obj;
use core::clock::Seconds;

#[derive(Copy, Clone)]
pub enum WorldEvent {
	UserClick,
	Die(obj::Id),
}

#[derive(Copy, Clone)]
pub struct WorldEventNotification {
	timestamp: Seconds,
	event: WorldEvent,
}

impl WorldEventNotification {
	pub fn new(timestamp: Seconds, event: WorldEvent) -> Self {
		WorldEventNotification { timestamp, event }
	}
}
