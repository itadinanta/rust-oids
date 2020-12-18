use super::*;
use app::constants::*;
use app::Event;
use backend::messagebus::{Inbox, Message, PubSub, ReceiveDrain, Whiteboard};
use backend::obj::Transformable;
use backend::world;
use backend::world::agent;
use cgmath::InnerSpace;
use core::clock::*;
use core::geometry::Transform;
use core::geometry::*;
use core::math::{exponential_filter, ExponentialFilter};
use rand;
use rand::Rng;
use std::f32::consts;

#[derive(Default)]
pub struct PlayerState {
	trigger_held: bool,
	bullet_speed: f32,
	bullet_ready: bool,
	firing_rate: SecondsValue,
	bullet_charge: SecondsValue,
}

pub struct GameSystem {
	timer: SimulationTimer,
	dt: Seconds,
	playerstate: PlayerState,
	feeders: Vec<Feeder>,
	inbox: Option<Inbox>,
}

struct Feeder {
	angle: Angle,
	position: Position,
	hourglass: Hourglass,
	light_intensity: ExponentialFilter<f32>,
	to_spawn: usize,
	spawned: usize,
	spin: Spin,
	emitted_spin: Spin,
	emitted_velocity: f32,
}

impl Feeder //where
{
	fn new<T>(position: Position, rate: Seconds, timer: &T) -> Self
	where T: Timer {
		Feeder {
			angle: 0.,
			position,
			light_intensity: exponential_filter(0., 0., EMITTER_INTENSITY_DECAY),
			hourglass: Hourglass::new(rate, timer),
			to_spawn: 0,
			spawned: 0,
			spin: consts::PI * 0.25,
			emitted_spin: consts::PI,
			emitted_velocity: 5.,
		}
	}
}

impl System for GameSystem {
	fn attach(&mut self, bus: &mut PubSub) {
		self.inbox = Some(bus.subscribe(Box::new(|ev| matches!(*ev, Message::Event(Event::PrimaryFire(_, _))))));
	}

	fn clear(&mut self) {
		self.playerstate = PlayerState::default();
		self.feeders = Vec::new();
	}

	fn import(&mut self, world: &world::World) {
		let messages = match self.inbox {
			Some(ref m) => m.drain(),
			None => Vec::new(),
		};

		for message in messages {
			if let Message::Event(Event::PrimaryFire(bullet_speed, rate)) = message {
				self.primary_fire(bullet_speed, rate);
			}
		}

		let source = world.feeders();
		// Add missing emitters - deletion not supported
		for s in &source[self.feeders.len()..] {
			//			let s = &source[i];
			self.feeders.push(Feeder::new(s.transform().position, s.rate(), &self.timer));
		}
		for (i, d) in self.feeders.iter_mut().enumerate() {
			d.position = source[i].transform().position;
		}
	}

	fn update(&mut self, _: &dyn world::AgentState, dt: Seconds) {
		let rng = &mut rand::thread_rng();
		self.dt = dt;

		self.timer.tick(dt);
		for e in &mut self.feeders {
			e.spawned = e.to_spawn;
		}
		for e in &mut self.feeders {
			if e.hourglass.is_expired(&self.timer) {
				e.hourglass.flip(&self.timer);
				e.hourglass.delay(seconds(rng.next_f32() * EMITTER_SPREAD_JITTER));
				e.light_intensity.force_to(1.0);
				e.to_spawn += 1;
			}
			e.light_intensity.update(dt.get() as f32);
			let tangent = Position::new(-e.position.y, e.position.x).normalize();
			e.angle += dt * e.spin;
			e.position += tangent * (dt * rng.next_f32());
		}
		// Byzantine way of processing trigger presses without trigger releases
		// I should think of something less convoluted
		if !self.playerstate.trigger_held {
			self.playerstate.bullet_charge = BULLET_FULL_CHARGE;
		}
		self.playerstate.bullet_ready =
			self.playerstate.trigger_held && self.playerstate.bullet_charge >= BULLET_FULL_CHARGE;
		self.playerstate.bullet_charge = if self.playerstate.bullet_ready {
			0.
		} else {
			BULLET_FULL_CHARGE
				.min(self.playerstate.bullet_charge + dt.get() * BULLET_FIRE_RATE * self.playerstate.firing_rate)
		};
		self.playerstate.trigger_held = false;
	}

	fn export(&self, world: &mut world::World, outbox: &dyn Outbox) {
		for e in &self.feeders {
			for _ in e.spawned..e.to_spawn {
				let r = e.angle;

				world.new_resource(
					Transform::new(e.position, r),
					Motion::new(Velocity::new(r.cos(), r.sin()) * e.emitted_velocity, e.emitted_spin),
				);
			}
		}

		for (src, dest) in self.feeders.iter().zip(world.feeders_mut().iter_mut()) {
			dest.transform_to(Transform::new(src.position, src.angle));
			dest.set_intensity(src.light_intensity.get());
		}

		if self.playerstate.bullet_ready {
			world.primary_fire(outbox, self.playerstate.bullet_speed);
		}

		world.get_player_agent_id().map(|player_id| {
			world.agent_mut(player_id).map(|agent| {
				agent.state.absorb(self.playerstate.bullet_charge as f32 * 100.0f32);
				if self.playerstate.bullet_ready {
					agent.reset_body_charge()
				}
			})
		});

		// if there are no minions, spawn some
		if world.agents(agent::AgentType::Minion).is_empty() {
			world.init_minions();
		}

		// if there are no players, spawn one
		if world.agents(agent::AgentType::Player).is_empty() {
			world.init_players();
		}
		for (_, agent) in world.agents_mut(agent::AgentType::Player).iter_mut() {
			for segment in agent.segments.iter_mut() {
				segment.state.update(self.dt);
			}
		}
	}
}

impl Default for GameSystem {
	fn default() -> Self {
		GameSystem {
			timer: SimulationTimer::new(),
			dt: seconds(0.),
			playerstate: PlayerState::default(),
			feeders: Vec::new(),
			inbox: None,
		}
	}
}

impl GameSystem {
	fn primary_fire(&mut self, bullet_speed: f32, firing_rate: SecondsValue) {
		self.playerstate.bullet_speed = bullet_speed;
		self.playerstate.firing_rate = firing_rate;
		self.playerstate.trigger_held = true;
	}
}
