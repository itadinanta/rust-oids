use backend::world;

pub trait AlertPlayer {
	fn play(&mut self, alert: &world::AlertEvent);
}
