use backend::obj::*;
use std::collections::HashMap;
use std::collections::HashSet;
use rand;
use core::geometry::*;
use backend::world::phen;
use backend::world::agent;
use backend::world::agent::Agent;
use backend::world::agent::AgentType;
use backend::world::agent::TypedAgent;
use backend::world::gen::*;

pub struct Swarm {
	seq: Id,
	agent_type: AgentType,
	gen: Genome,
	agents: agent::AgentMap,
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

	pub fn spawn<T>(&mut self, transform: Transform, motion: Option<Motion>, charge: f32) -> Id
		where T: phen::Phenotype {
		let id = self.next_id();
		match id.type_of() {
			t @ AgentType::Minion => println!("spawn: {} as {}", self.gen, t),
			_ => {}
		}
		let entity = T::develop(&mut self.gen, id, transform, motion, charge);
		self.mutate(&mut rand::thread_rng());
		self.insert(entity)
	}

	pub fn replicate<T>(&mut self,
	                    genome: &mut Genome,
	                    transform: Transform,
	                    motion: Option<Motion>,
	                    charge: f32)
	                    -> Id
		where T: phen::Phenotype {
		let id = self.next_id();
		println!("replicate: {} as {}", genome, id.type_of());
		let entity = T::develop(genome, id, transform, motion, charge);
		self.insert(entity)
	}

	fn insert(&mut self, agent: Agent) -> Id {
		let id = agent.id();
		self.agents.insert(id, agent);
		id
	}

	pub fn agents(&self) -> &HashMap<Id, Agent> {
		&self.agents
	}

	pub fn agents_mut(&mut self) -> &mut HashMap<Id, Agent> {
		&mut self.agents
	}
}

pub type SwarmMap = HashMap<AgentType, Swarm>;
