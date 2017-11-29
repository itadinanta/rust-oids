use conrod;
use std;

pub fn default_theme() -> conrod::Theme {
	use conrod::position::{Align, Direction, Padding, Position, Relative};
	conrod::Theme {
		name: "Demo Theme".to_string(),
		padding: Padding::none(),
		x_position: Position::Relative(Relative::Align(Align::Start), None),
		y_position: Position::Relative(Relative::Direction(Direction::Backwards, 20.0), None),
		background_color: conrod::color::DARK_CHARCOAL,
		shape_color: conrod::color::LIGHT_CHARCOAL,
		border_color: conrod::color::BLACK,
		border_width: 0.0,
		label_color: conrod::color::WHITE,
		font_id: None,
		font_size_large: 26,
		font_size_medium: 18,
		font_size_small: 12,
		widget_styling: conrod::theme::StyleMap::default(),
		mouse_drag_threshold: 0.0,
		double_click_threshold: std::time::Duration::from_millis(500),
	}
}