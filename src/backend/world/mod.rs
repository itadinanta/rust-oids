pub mod segment;
pub mod agent;

use backend::obj;
use backend::obj::*;
use rand;
use rand::Rng;
use std::collections::HashMap;
use std::collections::HashSet;
use std::f32::consts;
use num;
use core::color;
use core::color::ToRgb;
use core::geometry::*;
use backend::world::segment::*;
use backend::world::agent::Agent;

pub struct Swarm {
	seq: Id,
	rnd: Randomizer,
	agents: HashMap<Id, agent::Agent>,
}

struct Randomizer {
	rng: rand::ThreadRng,
}

#[allow(dead_code)]
impl Randomizer {
	fn new() -> Self {
		Randomizer { rng: rand::thread_rng() }
	}

	fn frand<T>(&mut self, min: T, max: T) -> T
		where T: rand::Rand + num::Float {
		self.rng.gen::<T>() * (max - min) + min
	}

	fn irand<T>(&mut self, min: T, max: T) -> T
		where T: rand::Rand + num::Integer + Copy {
		self.rng.gen::<T>() % (max - min + T::one()) + min
	}

	fn random_ball(&mut self) -> Shape {
		let radius: f32 = self.frand(1.0, 2.0);
		Shape::new_ball(radius)
	}

	fn random_box(&mut self) -> Shape {
		let radius: f32 = self.frand(1.0, 2.0);
		let ratio: f32 = self.frand(1.0, 2.0);
		Shape::new_box(radius, ratio)
	}

	fn random_vbar(&mut self) -> Shape {
		let radius: f32 = self.frand(1.0, 2.0);
		let ratio: f32 = self.frand(0.1, 0.2);
		Shape::new_box(radius, ratio)
	}

	fn random_triangle(&mut self) -> Shape {
		let radius = self.frand(0.5, 1.0);
		let alpha1 = self.frand(consts::PI * 0.5, consts::PI * 0.9);
		let alpha2 = consts::PI * 1.5 - self.frand(0., consts::PI);
		Shape::new_triangle(radius, alpha1, alpha2)
	}

	fn random_iso_triangle(&mut self) -> Shape {
		let radius = self.frand(0.5, 1.0);
		let alpha1 = self.frand(consts::PI * 0.5, consts::PI * 0.9);
		let alpha2 = consts::PI * 2. - alpha1;
		Shape::new_triangle(radius, alpha1, alpha2)
	}

	fn random_eq_triangle(&mut self) -> Shape {
		let radius = self.frand(0.5, 1.0);
		let alpha1 = consts::PI * 2. / 3.;
		let alpha2 = consts::PI * 2. - alpha1;
		Shape::new_triangle(radius, alpha1, alpha2)
	}

	fn random_star(&mut self) -> Shape {
		let radius: f32 = self.frand(1.0, 2.0);
		let n = self.irand(3, 8);
		let ratio1 = self.frand(0.5, 1.0);
		let ratio2 = self.frand(0.7, 0.9) * (1. / ratio1);
		Shape::new_star(n, radius, ratio1, ratio2)
	}

	fn random_poly(&mut self, upside_down: bool) -> Shape {
		let n = self.irand(3, 8);
		self.random_npoly(n, upside_down)
	}

	fn random_npoly(&mut self, n: AttachmentIndex, upside_down: bool) -> Shape {
		let radius: f32 = self.frand(1.0, 2.0);
		let ratio1 = f32::cos(consts::PI / n as f32);
		let ratio2 = 1. / ratio1;
		if upside_down {
			Shape::new_star(n, radius * ratio1, ratio2, ratio1)
		} else {
			Shape::new_star(n, radius, ratio1, ratio2)
		}
	}
}

impl Swarm {
	pub fn new() -> Swarm {
		Swarm {
			seq: 0,
			rnd: Randomizer::new(),
			agents: HashMap::new(),
		}
	}

	pub fn get(&self, id: Id) -> Option<&Agent> {
		self.agents.get(&id)
	}

	pub fn get_mut(&mut self, id: Id) -> Option<&mut agent::Agent> {
		self.agents.get_mut(&id)
	}

	pub fn next_id(&mut self) -> Id {
		self.seq = self.seq + 1;
		self.seq
	}

	pub fn new_resource(&mut self, initial_pos: Position, charge: f32) -> Id {
		let albedo = color::YPbPr::new(0.5, self.rnd.frand(-0.5, 0.5), self.rnd.frand(-0.5, 0.5));
		let ball = self.rnd.random_ball();
		let mut builder = agent::AgentBuilder::new(self.next_id(),
		                                           Material { density: 1.0, ..Default::default() },
		                                           Livery { albedo: albedo.to_rgba(), ..Default::default() },
		                                           segment::State::with_charge(charge, 0., charge));
		self.insert(builder.start(initial_pos, 0., &ball).build())
	}

	pub fn free_resources(&mut self, freed: &mut Vec<Agent>) {
		let mut dead = HashSet::new();

		for id in self.agents
			.iter()
			.filter(|&(_, agent)| !agent.state.is_alive())
			.map(|(&id, _)| id) {
			dead.insert(id);
		}
		for id in &dead {
			if let Some(agent) = self.agents.remove(&id) {
				freed.push(agent);
			}
		}
	}

	pub fn new_minion(&mut self, initial_pos: Position, charge: f32) -> Id {
		let albedo = color::Hsl::new(self.rnd.frand(0., 1.), 0.5, 0.5);
		let mut builder = agent::AgentBuilder::new(self.next_id(),
		                                           Material { density: 0.2, ..Default::default() },
		                                           Livery { albedo: albedo.to_rgba(), ..Default::default() },
		                                           segment::State::with_charge(0., charge, charge));
		let arm_shape = self.rnd.random_star();
		let leg_shape = self.rnd.random_star();
		let torso_shape = self.rnd.random_npoly(5, true);
		let head_shape = self.rnd.random_iso_triangle();
		let tail_shape = self.rnd.random_vbar();
		let initial_angle = consts::PI / 2. + f32::atan2(initial_pos.y, initial_pos.x);

		let torso = builder.start(initial_pos, initial_angle, &torso_shape)
			.index();
		builder.addr(torso, 2, &arm_shape, ARM | JOINT | ACTUATOR | RUDDER)
			.addl(torso, -2, &arm_shape, ARM | JOINT | ACTUATOR | RUDDER);

		let head = builder.add(torso, 0, &head_shape, HEAD | SENSOR).index();
		builder.addr(head, 1, &head_shape, HEAD | ACTUATOR | RUDDER)
			.addl(head, 2, &head_shape, HEAD | ACTUATOR | RUDDER);

		let mut belly = torso;
		let mut belly_mid = torso_shape.mid();
		for _ in 0..self.rnd.irand(0, 4) {
			let belly_shape = self.rnd.random_poly(true);

			belly = builder.add(belly, belly_mid, &belly_shape, BELLY | JOINT).index();
			belly_mid = belly_shape.mid();
			if self.rnd.irand(0, 4) == 0 {
				builder.addr(belly, 2, &arm_shape, ARM | ACTUATOR | RUDDER)
					.addl(belly, -2, &arm_shape, ARM | ACTUATOR | RUDDER);
			}
		}

		builder.addr(belly, belly_mid - 1, &leg_shape, LEG | ACTUATOR | THRUSTER)
			.addl(belly,
			      -(belly_mid - 1),
			      &leg_shape,
			      LEG | ACTUATOR | THRUSTER)
			.add(belly, belly_mid, &tail_shape, TAIL | ACTUATOR | BRAKE);

		self.insert(builder.build())
	}

	fn insert(&mut self, agent: Agent) -> Id {
		let id = agent.id();
		self.agents.insert(id, agent);
		id
	}

	pub fn kill(&mut self, id: &Id) {
		if let Some(ref mut agent) = self.agents.get_mut(id) {
			agent.state.die();
		}
	}

	pub fn agents(&self) -> &HashMap<Id, Agent> {
		&self.agents
	}

	pub fn agents_mut(&mut self) -> &mut HashMap<Id, Agent> {
		&mut self.agents
	}
}

#[repr(packed)]
#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
pub struct AgentRefs {
	pub agent_id: obj::Id,
	pub segment_index: obj::SegmentIndex,
	pub bone_index: obj::BoneIndex,
}

impl Default for AgentRefs {
	fn default() -> AgentRefs {
		AgentRefs {
			agent_id: 0xdeadbeef,
			segment_index: 0,
			bone_index: 0,
		}
	}
}

impl AgentRefs {
	pub fn with_id(id: obj::Id) -> AgentRefs {
		AgentRefs { agent_id: id, ..Default::default() }
	}

	pub fn with_segment(id: obj::Id, segment_index: obj::SegmentIndex) -> AgentRefs {
		AgentRefs {
			agent_id: id,
			segment_index: segment_index,
			..Default::default()
		}
	}

	pub fn with_bone(id: obj::Id, segment_index: obj::SegmentIndex, bone_index: obj::BoneIndex) -> AgentRefs {
		AgentRefs {
			agent_id: id,
			segment_index: segment_index,
			bone_index: bone_index,
		}
	}

	pub fn no_bone(&self) -> AgentRefs {
		AgentRefs { bone_index: 0, ..*self }
	}
}

pub struct World {
	pub extent: Rect,
	pub fence: obj::Mesh,
	pub players: Swarm,
	pub minions: Swarm,
	pub friendly_fire: Swarm,
	pub enemies: Swarm,
	pub enemy_fire: Swarm,
	pub resources: Swarm,
	pub props: Swarm,
}

pub trait WorldState {
	fn minion(&self, id: obj::Id) -> Option<&Agent>;
}

impl WorldState for World {
	fn minion(&self, id: obj::Id) -> Option<&Agent> {
		self.minions.get(id)
	}
}

pub struct Cleanup {
	pub freed: Box<[Agent]>,
}

impl World {
	pub fn new() -> Self {
		World {
			extent: Rect::new(-50., -50., 50., 50.),
			fence: Mesh::from_shape(Shape::new_ball(50.), Winding::CW),
			players: Swarm::new(),
			minions: Swarm::new(),
			friendly_fire: Swarm::new(),
			enemies: Swarm::new(),
			enemy_fire: Swarm::new(),
			resources: Swarm::new(),
			props: Swarm::new(),
		}
	}

	pub fn new_resource(&mut self, pos: Position) -> obj::Id {
		self.minions.new_resource(pos, 0.8)
	}

	pub fn new_minion(&mut self, pos: Position) -> obj::Id {
		self.minions.new_minion(pos, 0.3)
	}

	pub fn friend_mut(&mut self, id: obj::Id) -> Option<&mut Agent> {
		self.minions.get_mut(id)
	}

	pub fn cleanup(&mut self) -> Cleanup {
		let mut v = Vec::new();
		self.minions.free_resources(&mut v);
		Cleanup { freed: v.into_boxed_slice() }
	}
}
