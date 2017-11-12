use backend::world;

pub trait AlertPlayer<T> {
	fn play(&mut self, alert: &T);
}

pub struct NullAlertPlayer {}

impl NullAlertPlayer {
	pub fn new() -> NullAlertPlayer {
		NullAlertPlayer {}
	}
}

impl<T> AlertPlayer<T> for NullAlertPlayer {
	fn play(&mut self, alert: &T) {
		// do nothing
	}
}