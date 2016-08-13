use cgmath::Vector2;

pub type Position = Vector2<f32>;
pub type Translation = Vector2<f32>;

#[derive(Clone, Default)]
pub struct Size {
	pub width: f32,
	pub height: f32,
}

#[derive(Copy, Clone)]
pub struct Transform {
	pub position: Position,
	pub angle: f32,
	pub scale: f32,
}

#[derive(Copy, Clone)]
pub struct Rect {
	pub min: Position,
	pub max: Position,
}

impl Default for Transform {
	fn default() -> Transform {
		Transform {
			position: Position::new(0., 0.),
			angle: 0.,
			scale: 1.,
		}
	}
}
impl Transform {
	pub fn new(position: Position, angle: f32) -> Self {
		Transform {
			position: position,
			angle: angle,
			..Transform::default()
		}
	}

	pub fn with_position(position: Position) -> Self {
		Transform { position: position, ..Transform::default() }
	}
}
