use backend::obj;
use core::clock::Seconds;

#[derive(Copy, Clone)]
pub enum Alert {
	UserClick,
	Die(obj::Id),
}

#[derive(Copy, Clone)]
pub struct AlertEvent {
	timestamp: Seconds,
	alert: Alert,
}

impl AlertEvent {
	pub fn new(timestamp: Seconds, alert: Alert) -> Self {
		AlertEvent { timestamp, alert }
	}
}

