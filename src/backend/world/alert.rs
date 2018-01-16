use core::clock::Seconds;

#[allow(unused)]
#[derive(Copy, Clone, Debug)]
pub enum Alert {
	BeginSimulation,
	NewMinion,
	NewSpore,
	NewResource,
	NewBullet(usize),
	DieMinion,
	DieResource,
	Fertilised,
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

