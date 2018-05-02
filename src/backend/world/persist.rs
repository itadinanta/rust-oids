use std::io;
use std::fs;
use backend::world;
use backend::world::agent;
use backend::world::gen;
use num_traits::FromPrimitive;
use core::geometry;
use core::clock;
use serde_json;
use serialize::base64::{self, ToBase64, FromBase64};

#[derive(Serialize, Deserialize, Debug)]
pub struct Segment {
	charge: f32,
	target_charge: f32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Agent {
	id: usize,
	x: f32,
	y: f32,
	angle: f32,
	dna: String,
	age_seconds: f64,
	age_frames: usize,
	flags: u32,
	maturity: f32,
	phase: f32,
	energy: f32,
	segments: Vec<Segment>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Swarm {
	seq: usize,
	agent_type: usize,
	agents: Vec<Agent>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct World {
	left: f32,
	bottom: f32,
	right: f32,
	top: f32,
	swarms: Vec<Swarm>,
	regenerations: usize,
	minion_gene_pool: Vec<String>,
	minion_gene_pool_index: usize,
	resource_gene_pool: Vec<String>,
	resource_gene_pool_index: usize,
}

pub struct Serializer;

impl Serializer {
	pub fn to_world(world: &world::World) -> World {
		fn serialize_swarm(src: &world::swarm::Swarm, timer: &clock::SimulationTimer) -> Swarm {
			Swarm {
				seq: src.seq() as usize,
				agent_type: src.agent_type() as usize,
				agents: src.agents().iter().map(|(_k, v)| serialize_agent(v, timer)).collect(),
			}
		}

		fn serialize_agent(src: &world::agent::Agent, timer: &clock::SimulationTimer) -> Agent {
			let body = &src.segments[0];

			Agent {
				id: src.id(),
				x: body.transform.position.x,
				y: body.transform.position.y,
				angle: body.transform.angle,
				dna: src.dna().to_base64(base64::STANDARD),
				age_seconds: body.state.age_seconds().into(),
				age_frames: body.state.age_frames(),
				maturity: body.state.maturity(),
				flags: src.state.flags().bits(),
				phase: src.state.phase(),
				energy: src.state.energy(),
				segments: src.segments().iter().map(|s| serialize_segment(s)).collect(),
			}
		}

		fn serialize_segment(src: &world::segment::Segment) -> Segment {
			Segment {
				charge: src.state.get_charge(),
				target_charge: src.state.target_charge(),
			}
		}

		let swarms = world.swarms()
			.iter()
			.map(|(_k, v)| serialize_swarm(v, &world.clock))
			.collect();
		let minion_gene_pool: Vec<_> = world.minion_gene_pool
			.gene_pool_iter()
			.map(|dna| dna.to_base64(base64::STANDARD))
			.collect();
		let resource_gene_pool: Vec<_> = world.resource_gene_pool
			.gene_pool_iter()
			.map(|dna| dna.to_base64(base64::STANDARD))
			.collect();
		World {
			left: world.extent.min.x,
			bottom: world.extent.min.y,
			right: world.extent.max.x,
			top: world.extent.max.y,
			swarms,
			regenerations: world.regenerations,
			minion_gene_pool,
			minion_gene_pool_index: world.minion_gene_pool.gene_pool_index(),
			resource_gene_pool,
			resource_gene_pool_index: world.resource_gene_pool.gene_pool_index(),
		}
	}

	pub fn from_world(src: World, world: &mut world::World) {
		let timer = world.clock.clone();
		world.extent.min.x = src.left;
		world.extent.min.y = src.bottom;
		world.extent.max.x = src.right;
		world.extent.max.y = src.top;
		world.regenerations = src.regenerations;

		world.minion_gene_pool.populate_from_base64(&src.minion_gene_pool, src.minion_gene_pool_index);
		world.resource_gene_pool.populate_from_base64(&src.resource_gene_pool, src.resource_gene_pool_index);

		let mut registered = Vec::new();
		for src_swarm in src.swarms.iter() {
			if let Some(agent_type) = agent::AgentType::from_usize(src_swarm.agent_type) {
				let swarm = world.swarm_mut(&agent_type);
				swarm.reset(src_swarm.seq);
				for src_agent in &src_swarm.agents {
					if let Ok(dna) = src_agent.dna.from_base64() {
						let initial_transform = geometry::Transform::new(geometry::Position::new(src_agent.x, src_agent.y),
																		 src_agent.angle);
						let initial_maturity = src_agent.maturity;
						let id = swarm.rebuild(src_agent.id, &mut gen::Genome::new(dna), initial_transform, initial_maturity, &timer);
						if let Some(agent) = swarm.get_mut(id) {
							agent.state.restore(src_agent.flags, src_agent.phase, src_agent.energy);

							for (src_segment, dest_segment) in src_agent.segments.iter().zip(agent.segments_mut().iter_mut()) {
								//dest_segment.transform = geometry::Transform::new(geometry::Position::new(src_segment.x, src_segment.y),
								//												  src_segment.angle);
								dest_segment.state.restore(src_segment.charge, src_segment.target_charge, src_agent.age_seconds, src_agent.age_frames);
							};
							registered.push(id);
						}
					}
				}
			}
		}
		world.registered_player_id = world.swarms().get(&agent::AgentType::Player)
			.and_then(|swarm| swarm.agents().iter().next().map(|(k, _s)| *k));
		for id in registered {
			world.register(id);
		}
	}

	#[allow(unused)]
	pub fn from_string(source: &str, dest: &mut world::World) -> Result<(), serde_json::Error> {
		let result: Result<World, _> = serde_json::from_str(source);
		match result {
			Ok(src) => {
				Self::from_world(src, dest);
				Ok(())
			}
			Err(e) => Err(e)
		}
	}

	#[allow(unused)]
	pub fn to_string(world: &world::World) -> Result<String, serde_json::Error> {
		let s_world = Self::to_world(world);
		serde_json::to_string_pretty(&s_world)
	}

	pub fn save(file_path: &str, world: &world::World) -> io::Result<()> {
		let out_file = fs::File::create(file_path)?;
		let s_world = Self::to_world(world);
		serde_json::to_writer_pretty(out_file, &s_world)?;
		Ok(())
	}

	pub fn load(file_path: &str, world: &mut world::World) -> io::Result<()> {
		let in_file = fs::File::open(file_path)?;
		let src = serde_json::from_reader(in_file)?;
		Self::from_world(src, world);
		Ok(())
	}
}
