use super::*;

pub struct AudioSystem {}

impl Updateable for AudioSystem {}

impl System for AudioSystem {}

impl Default for AudioSystem {
	fn default() -> Self {
		AudioSystem {}
	}
}

impl AudioSystem {}
