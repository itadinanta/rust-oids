use super::geometry::*;

pub trait ViewTransform {
	fn to_view(&self, screen_position: Position) -> Position;
}

pub trait WorldTransform {
	fn to_world(&self, view_position: Position) -> Position;
}

pub struct Viewport {
	width: u32,
	height: u32,
	pub ratio: f32,
	pub scale: f32,
}

impl Viewport {
	pub fn rect(w: u32, h: u32, scale: f32) -> Viewport {
		Viewport {
			width: w,
			height: h,
			ratio: (w as f32 / h as f32),
			scale,
		}
	}
}

impl WorldTransform for Viewport {
	fn to_world(&self, pos: Position) -> Position {
		let dx = self.width as f32 / self.scale;
		let tx = (pos.x - (self.width as f32 * 0.5)) / dx;
		let ty = ((self.height as f32 * 0.5) - pos.y) / dx;
		Position::new(tx, ty)
	}
}

impl ViewTransform for Viewport {
	fn to_view(&self, screen_position: Position) -> Position {
		self.to_world(screen_position)
	}
}
