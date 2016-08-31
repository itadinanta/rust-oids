pub mod segment;
pub mod agent;
pub mod swarm;
pub mod gen;
pub mod phen;

use backend::obj;
use backend::obj::*;
use std::collections::HashMap;
use std::collections::HashSet;
use core::geometry::*;
use backend::world::agent::Agent;
use backend::world::agent::AgentType;
use backend::world::agent::TypedAgent;
use backend::world::swarm::*;

pub struct World {
	pub extent: Rect,
	pub fence: obj::Mesh,
	swarms: HashMap<AgentType, Swarm>,
	registered: HashSet<Id>,
}

pub trait WorldState {
	fn agent(&self, id: obj::Id) -> Option<&Agent>;
}

impl WorldState for World {
	fn agent(&self, id: obj::Id) -> Option<&Agent> {
		self.swarms.get(&id.type_of()).and_then(|m| m.get(id))
	}
}

pub struct Sweep {
	pub freed: Box<[Agent]>,
}

impl World {
	pub fn new() -> Self {
		let mut swarms = HashMap::new();
		let types = AgentType::all();
		for t in types {
			swarms.insert(*t, Swarm::new(*t));
		}
		World {
			extent: Rect::new(-50., -50., 50., 50.),
			fence: Mesh::from_shape(Shape::new_ball(50.), Winding::CW),
			swarms: swarms,
			registered: HashSet::new(),
		}
	}

	pub fn new_resource(&mut self, pos: Position, vel: Option<Motion>) -> obj::Id {
		let id = self.swarm_mut(&AgentType::Resource).spawn::<phen::Resource>(pos, vel, 0.8);
		self.register(id)
	}

	pub fn new_spore(&mut self, pos: Position, vel: Option<Motion>) -> obj::Id {
		let id = self.swarm_mut(&AgentType::Minion).spawn::<phen::Resource>(pos, vel, 0.8);
		self.register(id)
	}

	pub fn new_minion(&mut self, pos: Position, vel: Option<Motion>) -> obj::Id {
		let id = self.swarm_mut(&AgentType::Minion).spawn::<phen::Minion>(pos, vel, 0.3);
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
		self.swarms.get_mut(&id.type_of()).and_then(|m| m.get_mut(id))
	}

	pub fn agents_mut(&mut self, agent_type: AgentType) -> &mut agent::AgentMap {
		self.swarms.get_mut(&agent_type).unwrap().agents_mut()
	}

	pub fn swarm_mut(&mut self, agent_type: &AgentType) -> &mut Swarm {
		self.swarms.get_mut(&agent_type).unwrap()
	}

	pub fn swarms(&self) -> &SwarmMap {
		&self.swarms
	}

	pub fn sweep(&mut self) -> Sweep {
		let mut v = Vec::new();
		for (_, agents) in self.swarms.iter_mut() {
			agents.free_resources(&mut v);
		}
		Sweep { freed: v.into_boxed_slice() }
	}
}
