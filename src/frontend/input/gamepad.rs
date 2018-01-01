use gilrs::Gilrs;

#[allow(unused)]
pub enum Button {
	North,
	East,
	South,
	West,
	Up,
	Down,
	Left,
	Right,
	RT,
	LT,
	RB,
	LB,
	RC,
	LC,
}

pub struct Gamepad {
	gilrs: Gilrs,
}

impl Gamepad {
	pub fn new() -> Self {
		Gamepad {
			gilrs: Gilrs::new()
		}
	}
}
