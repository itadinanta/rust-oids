pub trait AlertPlayer<T, E> {
	fn play(&mut self, alert: &T) -> Result<(), E>;
}

pub struct NullAlertPlayer {}

impl NullAlertPlayer {
	pub fn new() -> NullAlertPlayer {
		NullAlertPlayer {}
	}
}

impl<T> AlertPlayer<T, ()> for NullAlertPlayer {
	fn play(&mut self, _: &T) -> Result<(), ()> {
		Ok(())
	}
}