pub mod segment;
pub mod agent;
pub mod gen;
pub mod phen;

use backend::obj;
use backend::obj::*;
use std::collections::HashMap;
use std::collections::HashSet;
use rand;
use core::geometry::*;
use backend::world::agent::Agent;
use backend::world::agent::AgentType;
use backend::world::gen::*;
use num::FromPrimitive;

pub struct Swarm {
	seq: Id,
	agent_type: AgentType,
	gen: Genome,
	agents: AgentMap,
}

impl Swarm {
	pub fn new(agent_type: AgentType) -> Swarm {
		Swarm {
			seq: 0,
			agent_type: agent_type,
			gen: Genome::new(b"Rust-Oids are cool!"),
			agents: HashMap::new(),
		}
	}

	#[allow(dead_code)]
	pub fn type_of(&self) -> AgentType {
		self.agent_type
	}

	pub fn get(&self, id: Id) -> Option<&Agent> {
		self.agents.get(&id)
	}

	pub fn get_mut(&mut self, id: Id) -> Option<&mut agent::Agent> {
		self.agents.get_mut(&id)
	}

	pub fn mutate<R: rand::Rng>(&mut self, rng: &mut R) {
		self.gen = self.gen.mutate(rng);
	}

	pub fn next_id(&mut self) -> Id {
		self.seq = self.seq + 1;
		self.seq << 8 | (self.agent_type as usize)
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

	pub fn spawn<T>(&mut self, initial_pos: Position, initial_vel: Option<Motion>, charge: f32) -> Id
		where T: phen::Phenotype {
		let id = self.next_id();
		let entity = T::develop(&mut self.gen, id, initial_pos, initial_vel, charge);
		self.mutate(&mut rand::thread_rng());
		self.insert(entity)
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


pub type AgentMap = HashMap<Id, agent::Agent>;
pub type SwarmMap = HashMap<AgentType, Swarm>;

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

pub trait TypedAgent {
	fn type_of(&self) -> AgentType;
}

impl TypedAgent for Id {
	fn type_of(&self) -> AgentType {
		AgentType::from_usize(*self & 0xff).unwrap_or(AgentType::Prop)
	}
}

pub struct World {
	pub extent: Rect,
	pub fence: obj::Mesh,
	agents: HashMap<AgentType, Swarm>,
	registered: HashSet<Id>,
}

pub trait WorldState {
	fn agent(&self, id: obj::Id) -> Option<&Agent>;
}

impl WorldState for World {
	fn agent(&self, id: obj::Id) -> Option<&Agent> {
		self.agents.get(&id.type_of()).and_then(|m| m.get(id))
	}
}

pub struct Cleanup {
	pub freed: Box<[Agent]>,
}

impl World {
	pub fn new() -> Self {
		let mut agents = HashMap::new();
		let types = AgentType::all();
		for t in types {
			agents.insert(*t, Swarm::new(*t));
		}
		World {
			extent: Rect::new(-50., -50., 50., 50.),
			fence: Mesh::from_shape(Shape::new_ball(50.), Winding::CW),
			agents: agents,
			registered: HashSet::new(),
		}
	}

	pub fn new_resource(&mut self, pos: Position, vel: Option<Motion>) -> obj::Id {
		let id = self.agents.get_mut(&AgentType::Resource).unwrap().spawn::<phen::Resource>(pos, vel, 0.8);
		self.register(id)
	}

	pub fn new_spore(&mut self, pos: Position, vel: Option<Motion>) -> obj::Id {
		let id = self.agents.get_mut(&AgentType::Minion).unwrap().spawn::<phen::Resource>(pos, vel, 0.8);
		self.register(id)
	}

	pub fn new_minion(&mut self, pos: Position, vel: Option<Motion>) -> obj::Id {
		let id = self.agents.get_mut(&AgentType::Minion).unwrap().spawn::<phen::Minion>(pos, vel, 0.3);
		self.register(id)
	}

	pub fn register(&mut self, id: obj::Id) -> obj::Id {
		self.registered.insert(id);
		id
	}

	pub fn registered(&mut self) -> Box<[Id]> {
		let collection = self.registered.iter().map(|r| *r).collect::<Vec<_>>().into_boxed_slice();
		self.registered.clear();
		collection
	}

	pub fn agent_mut(&mut self, id: obj::Id) -> Option<&mut Agent> {
		self.agents.get_mut(&id.type_of()).and_then(|m| m.get_mut(id))
	}

	pub fn agents_mut(&mut self, agent_type: AgentType) -> &mut AgentMap {
		&mut self.agents.get_mut(&agent_type).unwrap().agents
	}

	pub fn swarms(&self) -> &SwarmMap {
		&self.agents
	}

	pub fn swarms_mut(&mut self) -> &mut SwarmMap {
		&mut self.agents
	}

	pub fn cleanup(&mut self) -> Cleanup {
		let mut v = Vec::new();
		for (_, agents) in self.agents.iter_mut() {
			agents.free_resources(&mut v);
		}
		Cleanup { freed: v.into_boxed_slice() }
	}
}
