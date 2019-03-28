pub mod conrod_gfx;
pub mod conrod_ui;
pub mod theme;

use app::FrameUpdate;

pub trait AlertPlayer<T, E> {
	fn play(&mut self, alert: &T) -> Result<(), E>;
}

pub struct NullAlertPlayer {}

impl NullAlertPlayer {
	pub fn new() -> NullAlertPlayer { NullAlertPlayer {} }
}

impl<T> AlertPlayer<T, ()> for NullAlertPlayer {
	fn play(&mut self, _: &T) -> Result<(), ()> { Ok(()) }
}

#[derive(Debug, Copy, Clone)]
pub enum Error {
	FontLoader,
	ResourceLoader,
}

#[derive(Debug, Clone)]
pub enum Screen {
	#[allow(unused)]
	Help,
	Main(FrameUpdate),
}
