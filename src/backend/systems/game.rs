use super::*;
use std::f32::consts;
use rand;
use rand::Rng;
use cgmath::InnerSpace;
use app::constants::*;
use app::Event;
use core::clock::*;
use core::geometry::*;
use core::geometry::Transform;
use backend::obj::Transformable;
use backend::world;
use backend::world::agent;
use backend::world::Emission;
use backend::messagebus::{PubSub, Inbox, Whiteboard, ReceiveDrain, Message};

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
	playerstate: PlayerState,
	feeders: Vec<Feeder>,
	inbox: Option<Inbox>,
}

struct Feeder {
	position: Position,
	hourglass: Hourglass,
	to_spawn: usize,
	spawned: usize,
	emission: Emission,
	spin: Spin,
	velocity: f32,
}

impl Feeder where {
	fn new<T>(position: Position, rate: Seconds, emission: Emission, timer: &T) -> Self where T: Timer {
		Feeder {
			position,
			hourglass: Hourglass::new(rate, timer),
			to_spawn: 0,
			spawned: 0,
			emission,
			spin: consts::PI,
			velocity: 5.,
		}
	}
}

impl System for GameSystem {
	fn attach(&mut self, bus: &mut PubSub) {
		self.inbox = Some(bus.subscribe(Box::new(|ev|
			if let &Message::Event(Event::PrimaryFire(_, _)) = ev { true } else { false })));
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
			match message {
				Message::Event(Event::PrimaryFire(bullet_speed, rate)) => {
					self.primary_fire(bullet_speed, rate);
				}
				_ => {}
			}
		}

		let source = world.feeders();
		// Add missing emitters - deletion not supported
		for i in self.feeders.len()..source.len() {
			let s = &source[i];
			self.feeders.push(Feeder::new(
				s.transform().position,
				s.rate(),
				s.emission(),
				&self.timer,
			));
		}
		for (i, d) in self.feeders.iter_mut().enumerate() {
			d.position = source[i].transform().position;
		}
	}

	fn update(&mut self, _: &world::AgentState, dt: Seconds) {
		let rng = &mut rand::thread_rng();
		self.timer.tick(dt);
		for e in &mut self.feeders {
			e.spawned = e.to_spawn;
		}
		for e in &mut self.feeders {
			if e.hourglass.is_expired(&self.timer) {
				e.hourglass.flip(&self.timer);
				e.to_spawn += 1;
			}
			let tangent = Position::new(-e.position.y, e.position.x).normalize();
			e.position += tangent * rng.next_f32() * dt.get() as f32;
		}
		// Byzantine way of processing trigger presses without trigger releases
		// I should think of something less convoluted
		if !self.playerstate.trigger_held {
			self.playerstate.bullet_charge = BULLET_FULL_CHARGE;
		}
		self.playerstate.bullet_ready = self.playerstate.trigger_held &&
			self.playerstate.bullet_charge >= BULLET_FULL_CHARGE;
		self.playerstate.bullet_charge = if self.playerstate.bullet_ready {
			0.
		} else {
			BULLET_FULL_CHARGE.min(self.playerstate.bullet_charge +
				dt.get() * BULLET_FIRE_RATE * self.playerstate.firing_rate)
		};

		self.playerstate.trigger_held = false;
	}

	fn export(&self, world: &mut world::World, outbox: &Outbox) {
		let rng = &mut rand::thread_rng();
		for e in &self.feeders {
			for i in e.spawned..e.to_spawn {
				let r = match e.emission {
					Emission::Random => rng.next_f32() * 2. * consts::PI,
					Emission::CCW(angle) => angle * i as f32,
					Emission::CW(angle) => -angle * i as f32,
				};
				world.new_resource(
					Transform::new(e.position, r),
					Motion::new(Velocity::new(r.cos(), r.sin()) * e.velocity, e.spin),
				);
			}
		}

		for (i, d) in self.feeders.iter().enumerate() {
			world.feeders_mut()[i].transform_to(Transform::from_position(d.position));
		}

		if self.playerstate.bullet_ready {
			world.primary_fire(outbox, self.playerstate.bullet_speed);
		}

		world.get_player_agent_id().map(
			|player_id| {
				world.agent_mut(player_id).map(
					|agent| {
						agent.state.absorb(self.playerstate.bullet_charge as f32 * 100.0f32);
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
	}
}

impl Default for GameSystem {
	fn default() -> Self {
		GameSystem {
			timer: SimulationTimer::new(),
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
