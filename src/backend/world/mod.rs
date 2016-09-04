pub mod segment;
pub mod agent;
pub mod swarm;
pub mod gen;
pub mod phen;

use backend::obj;
use backend::obj::*;
use rand;
use chrono::*;
use std::f32::consts;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io;
use std::io::Write;
use std::fs;
use serialize::base64::{self, ToBase64};

use core::geometry::*;
use backend::world::agent::Agent;
use backend::world::agent::AgentType;
use backend::world::agent::TypedAgent;
use backend::world::swarm::*;


pub struct World {
	pub extent: Rect,
	swarms: HashMap<AgentType, Swarm>,
	emitters: Vec<Emitter>,
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

#[derive(Clone)]
pub struct Emitter {
	position: Position,
	rate: f32,
}

impl Emitter {
	pub fn new(x: f32, y: f32, rate: f32) -> Self {
		Emitter {
			position: Position::new(x, y),
			rate: rate,
		}
	}
	pub fn rate(&self) -> f32 {
		self.rate
	}
}

impl Transformable for Emitter {
	fn transform(&self) -> Transform {
		Transform::from_position(self.position)
	}
	fn transform_to(&mut self, t: Transform) {
		self.position = t.position;
	}
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
			swarms: swarms,
			emitters: vec![Emitter::new(-20., -20., 0.4),
			               Emitter::new(-20., 20., 0.8),
			               Emitter::new(20., 20., 1.2),
			               Emitter::new(20., -20., 2.0)],
			registered: HashSet::new(),
		}
	}

	pub fn new_resource(&mut self, pos: Position, vel: Option<Motion>) -> obj::Id {
		let id = self.swarm_mut(&AgentType::Resource).spawn::<phen::Resource>(Transform::from_position(pos), vel, 0.8);
		self.register(id)
	}

	pub fn decay_to_resource(&mut self, transform: Transform, dna: &gen::Dna) -> obj::Id {
		let id = self.swarm_mut(&AgentType::Resource)
			.replicate::<phen::Resource>(&mut gen::Genome::new(dna), transform, None, 0.8);
		self.register(id)
	}

	pub fn new_spore(&mut self, transform: Transform, dna: &gen::Dna) -> obj::Id {
		let id = self.swarm_mut(&AgentType::Spore)
			.replicate::<phen::Spore>(&mut gen::Genome::new(dna).mutate(&mut rand::thread_rng()),
			                          transform,
			                          None,
			                          0.8);
		self.register(id)
	}

	pub fn hatch_spore(&mut self, transform: Transform, dna: &gen::Dna) -> obj::Id {
		let id = self.swarm_mut(&AgentType::Minion)
			.replicate::<phen::Minion>(&mut gen::Genome::new(dna), transform, None, 0.3);
		self.register(id)
	}

	pub fn new_minion(&mut self, pos: Position, vel: Option<Motion>) -> obj::Id {
		let angle = consts::PI / 2. + f32::atan2(pos.y, pos.x);
		let id = self.swarm_mut(&AgentType::Minion).spawn::<phen::Minion>(Transform::new(pos, angle), vel, 0.3);
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

	pub fn agent(&self, id: obj::Id) -> Option<&Agent> {
		self.swarms.get(&id.type_of()).and_then(|m| m.get(id))
	}

	pub fn agent_mut(&mut self, id: obj::Id) -> Option<&mut Agent> {
		self.swarms.get_mut(&id.type_of()).and_then(|m| m.get_mut(id))
	}

	pub fn agents(&self, agent_type: AgentType) -> &agent::AgentMap {
		self.swarms.get(&agent_type).unwrap().agents()
	}

	pub fn agents_mut(&mut self, agent_type: AgentType) -> &mut agent::AgentMap {
		self.swarms.get_mut(&agent_type).unwrap().agents_mut()
	}

	pub fn swarm_mut(&mut self, agent_type: &AgentType) -> &mut Swarm {
		self.swarms.get_mut(&agent_type).unwrap()
	}

	pub fn emitters(&self) -> &[Emitter] {
		self.emitters.as_slice()
	}

	pub fn swarms(&self) -> &SwarmMap {
		&self.swarms
	}

	pub fn sweep(&mut self) -> Box<[Agent]> {
		let mut v = Vec::new();
		for (_, agents) in self.swarms.iter_mut() {
			agents.free_resources(&mut v);
		}
		v.into_boxed_slice()
	}

	pub fn dump(&self) -> io::Result<String> {
		let now: DateTime<UTC> = UTC::now();
		let file_name = now.format("rust-oids_%Y%m%d_%H%M%S.log").to_string();
		let mut f = try!(fs::File::create(&file_name));
		for (_, swarm) in self.swarms.iter() {
			try!(f.write_fmt(format_args!("[{}]\n", swarm.type_of())));
			for (_, agent) in swarm.agents().iter() {
				try!(f.write_fmt(format_args!("{}\t{}\n",
				                              agent.id(),
				                              agent.dna().to_base64(base64::STANDARD))));
			}
		}
		Ok(file_name)
	}
}
