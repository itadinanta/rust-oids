use super::*;
use std::rc::Rc;
use std::cell::RefCell;
use backend::world::WorldState;
use core::clock::*;

#[allow(unused)]
pub struct AnimationSystem {
	speed: SpeedFactor,
	animation_timer: SharedTimer<SimulationTimer>,
	simulation_timer: SharedTimer<SimulationTimer>,
	animation_clock: TimerStopwatch<SimulationTimer>,
	simulation_clock: TimerStopwatch<SimulationTimer>,
}

impl Updateable for AnimationSystem {
	fn update(&mut self, _: &WorldState, dt: Seconds) {
		self.simulation_timer.borrow_mut().tick(dt);
		self.animation_timer.borrow_mut().tick(dt * self.speed);
	}
}

impl System for AnimationSystem {}

impl Default for AnimationSystem {
	fn default() -> Self {
		let animation_timer = Rc::new(RefCell::new(SimulationTimer::new()));
		let simulation_timer = Rc::new(RefCell::new(SimulationTimer::new()));
		AnimationSystem {
			speed: 1.0,
			simulation_clock: TimerStopwatch::new(animation_timer.clone()),
			animation_clock: TimerStopwatch::new(simulation_timer.clone()),
			animation_timer,
			simulation_timer,
		}
	}
}

impl AnimationSystem {}
