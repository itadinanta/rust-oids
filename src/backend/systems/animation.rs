use super::*;
use backend::world::AgentState;
use backend::world::agent;
use core::clock::{seconds, Seconds, SimulationTimer, TimerStopwatch, SpeedFactor};
use num_traits::clamp;

#[allow(unused)]
pub struct AnimationSystem {
	speed: SpeedFactor,
	heartbeat_scale: SpeedFactor,
	background_animation_speed: SpeedFactor,
	dt: Seconds,
	animation_timer: SimulationTimer,
	simulation_timer: SimulationTimer,
	animation_clock: TimerStopwatch,
	simulation_clock: TimerStopwatch,
}

impl System for AnimationSystem {
	fn update(&mut self, _: &AgentState, dt: Seconds) {
		self.dt = dt;
		self.simulation_timer.tick(dt);
		self.animation_timer.tick(dt * self.speed);
	}

	fn export(&self, world: &mut world::World, _outbox: &Outbox) {
		let phase = world.phase_mut()[1] as f64 + self.dt.get() * self.speed * self.heartbeat_scale * self.background_animation_speed;
		world.phase_mut()[0] = 0.5;
		world.phase_mut()[1] = (phase % 1e+3) as f32;
		for (_, agent) in &mut world.agents_mut(agent::AgentType::Minion).iter_mut() {
			if agent.state.is_active() {
				let energy = agent.state.energy();
				agent.state.heartbeat((self.dt.get() * self.speed * self.heartbeat_scale) as f32 * clamp(energy, 50.0f32, 200.0f32))
			}
		}
	}
}

impl Default for AnimationSystem {
	fn default() -> Self {
		let animation_timer = SimulationTimer::new();
		let simulation_timer = SimulationTimer::new();
		AnimationSystem {
			speed: 1.0,
			heartbeat_scale: 1.0 / 60.0,
			background_animation_speed: 0.25,
			dt: seconds(0.0),
			simulation_clock: TimerStopwatch::new(&simulation_timer),
			animation_clock: TimerStopwatch::new(&animation_timer),
			animation_timer,
			simulation_timer,
		}
	}
}

impl AnimationSystem {}
