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

use core::geometry::*;
use core::resource::ResourceLoader;
use backend::world::agent::Agent;
use backend::world::agent::AgentType;
use backend::world::agent::TypedAgent;
use backend::world::swarm::*;
use serialize::base64::{self, ToBase64};

pub struct World {
	pub extent: Rect,
	swarms: HashMap<AgentType, Swarm>,
	emitters: Vec<Emitter>,
	registered: HashSet<Id>,
	minion_gene_pool: gen::GenePool,
	resource_gene_pool: gen::GenePool,
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
	transform: Transform,
	rate: f32,
}

impl Emitter {
	pub fn new(x: f32, y: f32, rate: f32) -> Self {
		Emitter {
			transform: Transform::from_position(Position::new(x, y)),
			rate: rate,
		}
	}
	pub fn rate(&self) -> f32 {
		self.rate
	}
}

impl Transformable for Emitter {
	fn transform(&self) -> &Transform {
		&self.transform
	}
	fn transform_to(&mut self, t: &Transform) {
		self.transform.position = t.position;
		self.transform.angle = t.angle;
	}
}

impl World {
	pub fn new<R>(res: &R, minion_gene_pool: &str) -> Self
		where R: ResourceLoader<u8> {
		let mut swarms = HashMap::new();
		let types = AgentType::all();
		for t in types {
			swarms.insert(*t, Swarm::new(*t));
		}
		fn default_gene_pool(_: io::Error) -> gen::GenePool {
			gen::GenePool::parse_from_base64(&["AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
			                                   "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
			                                   "GzB2lQVwM00tTAm5gwajjf4wc0a5GzB2lQVwM00tTAm5gwajjf4wc0a5",
			                                   "GzB2lQdwM10vQEu5zwaPgDhfq2v8GzB2lQdwM10vQEu5zwaPgDhfq2v8"])
		}

		World {
			extent: Rect::new(-80., -80., 80., 80.),
			swarms: swarms,
			emitters: vec![Emitter::new(-20., -20., 0.4),
			               Emitter::new(-20., 20., 0.4),
			               Emitter::new(20., 20., 0.4),
			               Emitter::new(20., -20., 0.4)],
			minion_gene_pool: res.load(minion_gene_pool)
				.map(|data| gen::GenePool::parse_from_resource(&data))
				.unwrap_or_else(default_gene_pool),
			resource_gene_pool: gen::GenePool::parse_from_base64(&["GyA21QoQ", "M00sWS0M"]),
			registered: HashSet::new(),
		}
	}

	pub fn new_resource(&mut self, transform: &Transform, motion: Option<&Motion>) -> obj::Id {
		let mut gen = &mut self.resource_gene_pool.next();
		let id = self.swarm_mut(&AgentType::Resource)
			.spawn::<phen::Resource>(&mut gen, transform, motion, 0.8);
		self.register(id)
	}

	pub fn decay_to_resource(&mut self, transform: &Transform, dna: &gen::Dna) -> obj::Id {
		let id = self.swarm_mut(&AgentType::Resource)
			.spawn::<phen::Resource>(&mut gen::Genome::new(dna), transform, None, 0.8);
		self.register(id)
	}

	pub fn new_spore(&mut self, transform: &Transform, dna: &gen::Dna) -> obj::Id {
		let id = self.swarm_mut(&AgentType::Spore)
			.spawn::<phen::Spore>(&mut gen::Genome::new(dna).mutate(&mut rand::thread_rng()),
			                      transform,
			                      None,
			                      0.8);
		self.register(id)
	}

	pub fn hatch_spore(&mut self, transform: &Transform, dna: &gen::Dna) -> obj::Id {
		let id = self.swarm_mut(&AgentType::Minion)
			.spawn::<phen::Minion>(&mut gen::Genome::new(dna), transform, None, 0.3);
		self.register(id)
	}

	pub fn randomize_minion(&mut self, pos: Position, motion: Option<&Motion>) -> obj::Id {
		self.minion_gene_pool.randomize();
		self.new_minion(pos, motion)
	}

	pub fn init_minions(&mut self) {
		let n = self.minion_gene_pool.len();
		let mut r = self.extent.top_right().x * 0.25;
		let mut angle = 0.0f32;
		let angle_delta = consts::PI * 2. / 16. as f32;
		for _ in 0..n {
			let pos = Position::new(r * angle.cos(), r * angle.sin());
			let mut gen = self.minion_gene_pool.next();
			let id = self.swarm_mut(&AgentType::Minion)
				.spawn::<phen::Minion>(&mut gen,
				                       &Transform::new(pos, angle + consts::PI / 2.),
				                       None,
				                       0.3);
			self.register(id);
			angle += angle_delta;
			r += 1.;
		}
	}

	pub fn new_minion(&mut self, pos: Position, motion: Option<&Motion>) -> obj::Id {
		let angle = consts::PI / 2. + f32::atan2(pos.y, pos.x);
		let mut gen = self.minion_gene_pool.next();
		let id = self.swarm_mut(&AgentType::Minion)
			.spawn::<phen::Minion>(&mut gen, &Transform::new(pos, angle), motion, 0.3);
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

	#[allow(dead_code)]
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
		let file_name = now.format("resources/%Y%m%d_%H%M%S.csv").to_string();
		let mut f = try!(fs::File::create(&file_name));
		for (_, agent) in self.agents(agent::AgentType::Minion).iter() {
			info!("{}", agent.dna().to_base64(base64::STANDARD));
			try!(f.write_fmt(format_args!("{}\n", agent.dna().to_base64(base64::STANDARD))));
		}
		Ok(file_name)
	}
}
