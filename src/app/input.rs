//! Input state, including current mouse position and button click
//! TODO: add keyboard presses
use app::obj;

pub struct InputState {
	left_button_pressed: bool,
	mouse_position: obj::Position,
}

impl Default for InputState {
	fn default() -> Self {
		InputState {
			left_button_pressed: false,
			mouse_position: obj::Position::new(0., 0.),
		}
	}
}

impl InputState {
	pub fn left_button_press(&mut self) {
		self.left_button_pressed = true;
	}

	pub fn left_button_release(&mut self) {
		self.left_button_pressed = false;
	}

	pub fn left_button_pressed(&self) -> bool {
		self.left_button_pressed
	}

	pub fn mouse_position_at(&mut self, pos: obj::Position) {
		self.mouse_position = pos;
	}

	pub fn mouse_position(&self) -> obj::Position {
		self.mouse_position
	}
}
