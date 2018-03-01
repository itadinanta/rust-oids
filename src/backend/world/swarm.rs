use backend::obj::*;
use std::collections::HashMap;
use std::collections::HashSet;
use core::clock::Timer;
use core::geometry::*;
use core::geometry::Transform;
use backend::world::phen;
use backend::world::agent;
use backend::world::agent::Agent;
use backend::world::agent::AgentType;
use backend::world::agent::TypedAgent;
use backend::world::gen::*;

pub struct Swarm {
	seq: Id,
	agent_type: AgentType,
	agents: agent::AgentMap,
}

impl Swarm {
	pub fn new(agent_type: AgentType) -> Swarm {
		Swarm {
			seq: 0,
			agent_type,
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

	pub fn next_id(&mut self) -> Id {
		self.seq = self.seq + 1;
		self.seq << 8 | (self.agent_type as usize)
	}

	pub fn free_resources(&mut self, freed: &mut Vec<Agent>) {
		let mut dead = HashSet::new();
		for id in self.agents
			.iter()
			.filter(|&(_, agent)| !agent.state.is_alive())
			.map(|(&id, _)| id)
			{
				dead.insert(id);
			}
		for id in &dead {
			if let Some(agent) = self.agents.remove(&id) {
				freed.push(agent);
			}
		}
	}

	pub fn spawn<P, T>(&mut self, genome: &mut Genome, transform: Transform, motion: Option<&Motion>, charge: f32, timer: &T) -> Id
		where
			P: phen::Phenotype,
			T: Timer {
		let id = self.next_id();
		match id.type_of() {
			AgentType::Minion | AgentType::Spore => info!("spawn: {} as {}", genome, id.type_of()),
			_ => {}
		}
		let entity = P::develop(genome, id, transform, motion, charge, timer);
		self.insert(entity)
	}

	fn insert(&mut self, agent: Agent) -> Id {
		let id = agent.id();
		self.agents.insert(id, agent);
		id
	}

	#[allow(dead_code)]
	pub fn is_empty(&self) -> bool {
		self.agents.is_empty()
	}

	pub fn agents(&self) -> &HashMap<Id, Agent> {
		&self.agents
	}

	pub fn agents_mut(&mut self) -> &mut HashMap<Id, Agent> {
		&mut self.agents
	}
}

pub type SwarmMap = HashMap<AgentType, Swarm>;
