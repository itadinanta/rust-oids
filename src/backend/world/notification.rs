use backend::obj;

#[derive(Copy, Clone)]
pub enum WorldEvent {
	UserClick,
	Die(obj::Id),
}

#[derive(Copy, Clone)]
pub struct WorldEventNotification {
	timestamp: f32,
	event: WorldEvent,
}

impl WorldEventNotification {
	pub fn new(timestamp: f32, event: WorldEvent) -> Self {
		WorldEventNotification { timestamp, event }
	}
}
