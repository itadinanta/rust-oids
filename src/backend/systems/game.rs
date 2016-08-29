use super::*;

pub struct GameSystem {}

impl Updateable for GameSystem {}

impl System for GameSystem {}

impl Default for GameSystem {
	fn default() -> Self {
		GameSystem {}
	}
}

impl GameSystem {}
