use backend::obj::*;
use std::collections::HashMap;
use std::collections::HashSet;
use core::clock::Timer;
use backend::world::phen;
use backend::world::agent;
use backend::world::agent::Agent;
use backend::world::agent::AgentType;
use backend::world::agent::TypedAgent;
use backend::world::gen::*;

pub struct Swarm {
	seq: Id,
	agent_type: AgentType,
	phenotype: Box<phen::Phenotype>,
	agents: agent::AgentMap,
}

impl Swarm {
	pub fn new(agent_type: AgentType, phenotype: Box<phen::Phenotype>) -> Swarm {
		Swarm {
			seq: 0,
			agent_type,
			phenotype,
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

	pub fn agent_type(&self) -> AgentType { self.agent_type }

	pub fn seq(&self) -> Id { self.seq }

	pub fn reset(&mut self, seq: Id) {
		self.agents.clear();
		self.seq = seq;
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

	fn insert(&mut self, agent: Agent) -> Id {
		let id = agent.id();
		self.agents.insert(id, agent);
		id
	}

	#[allow(dead_code)]
	pub fn is_empty(&self) -> bool {
		self.agents.is_empty()
	}

	pub fn spawn(&mut self, genome: &mut Genome, initial_state: agent::InitialState, timer: &Timer) -> Id {
		let id = self.next_id();
		match id.type_of() {
			AgentType::Minion | AgentType::Spore => info!("spawn: {} as {}", genome, id.type_of()),
			_ => {}
		}
		// dynamic dispatch
		let entity = self.phenotype.develop(genome, id, initial_state, timer);
		self.insert(entity)
	}

	pub fn rebuild(&mut self, id: Id, genome: &mut Genome, initial_state: agent::InitialState, timer: &Timer) -> Id {
		let entity = self.phenotype.develop(genome, id, initial_state, timer);
		self.insert(entity)
	}

	pub fn agents(&self) -> &HashMap<Id, Agent> {
		&self.agents
	}

	pub fn agents_mut(&mut self) -> &mut HashMap<Id, Agent> {
		&mut self.agents
	}
}

pub type SwarmMap = HashMap<AgentType, Swarm>;
