use core::clock::Seconds;

#[allow(unused)]
#[derive(Copy, Clone, Debug)]
pub enum Alert {
	NewMinion,
	NewSpore,
	NewResource,
	DieMinion,
	DieResource,
}

#[derive(Copy, Clone, Debug)]
pub struct AlertEvent {
	pub timestamp: Seconds,
	pub alert: Alert,
}

impl AlertEvent {
	pub fn new(timestamp: Seconds, alert: Alert) -> Self {
		AlertEvent { timestamp, alert }
	}
}

