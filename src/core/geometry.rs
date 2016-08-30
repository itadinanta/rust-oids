use cgmath;
use cgmath::Vector2;

pub type Position = Vector2<f32>;
pub type Translation = Vector2<f32>;
pub type Velocity = Vector2<f32>;
pub type Angle = f32;
pub type Rotation = f32;
pub type Spin = f32;

pub type M44 = cgmath::Matrix4<f32>;

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
pub struct Motion {
	pub velocity: Velocity,
	pub spin: Spin,
}

#[derive(Copy, Clone)]
pub struct Rect {
	pub min: Position,
	pub max: Position,
}

impl Rect {
	pub fn new(left: f32, bottom: f32, right: f32, top: f32) -> Self {
		Rect {
			min: Position::new(left, bottom),
			max: Position::new(right, top),
		}
	}

	pub fn bottom_left(&self) -> Position {
		self.min
	}
	pub fn top_right(&self) -> Position {
		self.max
	}

	pub fn bottom_right(&self) -> Position {
		Position::new(self.max.x, self.min.y)
	}

	pub fn top_left(&self) -> Position {
		Position::new(self.min.x, self.max.y)
	}
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

pub fn origin() -> Position {
	Position::new(0., 0.)
}
