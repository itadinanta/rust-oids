use super::*;
use std::f32::consts;
use core::clock::*;
use core::geometry::*;
use backend::obj::Transformable;
use backend::world;

pub struct GameSystem {
	emitters: Vec<Emitter>,
}

struct Emitter {
	position: Position,
	hourglass: Hourglass<SystemStopwatch>,
	to_spawn: usize,
	spawned: usize,
	angle: Angle,
	spin: Spin,
	velocity: f32,
}

impl Emitter {
	fn new(position: Position, rate: f32) -> Self {
		Emitter {
			position: position,
			hourglass: Hourglass::new(rate),
			to_spawn: 0,
			spawned: 0,
			angle: consts::PI / 12.,
			spin: consts::PI,
			velocity: 10.,
		}
	}
}

impl Updateable for GameSystem {
	fn update(&mut self, _: &world::WorldState, _: f32) {
		for e in &mut self.emitters {
			e.spawned = e.to_spawn;
		}
		for e in &mut self.emitters {
			if e.hourglass.is_expired() {
				e.hourglass.flip();
				e.to_spawn += 1;
			}
		}
	}
}

impl System for GameSystem {
	fn from_world(&mut self, world: &world::World) {
		let source = world.emitters();
		// Add missing emitters - deletion not supported
		for i in self.emitters.len()..source.len() {
			let s = &source[i];
			self.emitters.push(Emitter::new(s.transform().position, s.rate()));
		}
		for (i, mut d) in self.emitters.iter_mut().enumerate() {
			d.position = source[i].transform().position;
		}
	}

	fn to_world(&self, world: &mut world::World) {
		for e in &self.emitters {
			for i in e.spawned..e.to_spawn {
				let r = e.angle * i as f32;
				world.new_resource(&Transform::new(e.position, r),
				                   Some(&Motion {
					                   velocity: Velocity::new(r.cos(), r.sin()) * e.velocity,
					                   spin: e.spin,
				                   }));
			}
		}
	}
}


impl Default for GameSystem {
	fn default() -> Self {
		GameSystem { emitters: Vec::new() }
	}
}

impl GameSystem {}
