//! Input state, including current mouse position and button click
//! TODO: add keyboard presses
use cgmath::Vector2;

pub enum Button {
	Left = 0,
	Right = 1,
}

pub struct InputState {
	button_pressed: [bool; 2],
	mouse_position: Vector2<f32>,
}

impl Default for InputState {
	fn default() -> Self {
		InputState {
			button_pressed: [false; 2],
			mouse_position: Vector2::new(0., 0.),
		}
	}
}

impl InputState {
	pub fn button_pressed(&self, b: Button) -> bool {
		self.button_pressed[b as usize]
	}

	pub fn button_press(&mut self, b: Button) {
		self.button_pressed[b as usize] = true;
	}

	pub fn button_release(&mut self, b: Button) {
		self.button_pressed[b as usize] = false;
	}

	pub fn left_button_press(&mut self) {
		self.button_press(Button::Left);
	}

	pub fn left_button_release(&mut self) {
		self.button_release(Button::Left);
	}

	pub fn left_button_pressed(&self) -> bool {
		self.button_pressed(Button::Left)
	}

	pub fn mouse_position_at(&mut self, pos: Vector2<f32>) {
		self.mouse_position = pos;
	}

	pub fn mouse_position(&self) -> Vector2<f32> {
		self.mouse_position
	}
}
