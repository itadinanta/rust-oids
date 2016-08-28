pub mod segment;
pub mod agent;
pub mod gen;

use backend::obj;
use backend::obj::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::f32::consts;
use core::color;
use rand;
use core::color::ToRgb;
use core::geometry::*;
use backend::world::segment::*;
use backend::world::agent::Agent;
use backend::world::gen::*;

pub struct Swarm {
	seq: Id,
	seq_tag: Id,
	rng: rand::ThreadRng,
	gen: Genome,
	agents: HashMap<Id, agent::Agent>,
}

impl Swarm {
	pub fn new(seq_tag: Id) -> Swarm {
		Swarm {
			seq: 0,
			seq_tag: seq_tag,
			rng: rand::thread_rng(),
			gen: Genome::new(b"Rust-Oids are cool!"),
			agents: HashMap::new(),
		}
	}

	pub fn get(&self, id: Id) -> Option<&Agent> {
		self.agents.get(&id)
	}

	pub fn get_mut(&mut self, id: Id) -> Option<&mut agent::Agent> {
		self.agents.get_mut(&id)
	}

	pub fn mutate(&mut self) {
		self.gen = self.gen.mutate(&mut self.rng);
	}

	pub fn next_id(&mut self) -> Id {
		self.seq = self.seq + 1;
		self.seq << 8 + self.seq_tag
	}

	pub fn new_resource(&mut self, initial_pos: Position, charge: f32) -> Id {
		let albedo = color::YPbPr::new(0.5,
		                               self.gen.next_float(-0.5, 0.5),
		                               self.gen.next_float(-0.5, 0.5));
		let ball = self.gen.ball();
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
		let albedo = color::Hsl::new(self.gen.next_float(0., 1.), 0.5, 0.5);
		let mut builder = agent::AgentBuilder::new(self.next_id(),
		                                           Material { density: 0.2, ..Default::default() },
		                                           Livery { albedo: albedo.to_rgba(), ..Default::default() },
		                                           segment::State::with_charge(0., charge, charge));
		let arm_shape = self.gen.star();
		let leg_shape = self.gen.star();
		let torso_shape = self.gen.npoly(5, true);
		let head_shape = self.gen.iso_triangle();
		let tail_shape = self.gen.vbar();
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
		for _ in 0..self.gen.next_integer(0, 4) {
			let belly_shape = self.gen.poly(true);

			belly = builder.add(belly, belly_mid, &belly_shape, BELLY | JOINT).index();
			belly_mid = belly_shape.mid();
			if self.gen.next_integer(0, 4) == 0 {
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
		self.mutate();
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
			minions: Swarm::new(0),
			players: Swarm::new(1),
			friendly_fire: Swarm::new(2),
			enemies: Swarm::new(3),
			enemy_fire: Swarm::new(4),
			resources: Swarm::new(5),
			props: Swarm::new(6),
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
