use backend::world;
use backend::world::agent;
use backend::world::gen;
use backend::world::segment;
use num_traits::FromPrimitive;
use core::geometry;
use core::clock;
use serde_json;
use serialize::base64::{self, ToBase64, FromBase64};

#[derive(Serialize, Deserialize, Debug)]
pub struct Segment {
	index: usize,
	x: f32,
	y: f32,
	angle: f32,
	age_seconds: f64,
	age_frames: usize,
	maturity: f32,
	charge: f32,
	target_charge: f32,
	recharge: f32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Agent {
	id: usize,
	dna: String,
	lifecycle: f64,
	flags: u32,
	phase: f32,
	energy: f32,
	growth: f32,
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
			Agent {
				id: src.id(),
				dna: src.dna().to_base64(base64::STANDARD),
				lifecycle: src.state.lifecycle().elapsed(timer).into(),
				flags: src.state.flags().bits(),
				phase: src.state.phase(),
				energy: src.state.energy(),
				growth: src.state.growth(),
				segments: src.segments().iter().map(|s| serialize_segment(s)).collect(),
			}
		}

		fn serialize_segment(src: &world::segment::Segment) -> Segment {
			Segment {
				x: src.transform.position.x,
				y: src.transform.position.y,
				angle: src.transform.angle,
				index: src.index as usize,
				age_seconds: src.state.age_seconds().into(),
				age_frames: src.state.age_frames(),
				maturity: src.state.maturity(),
				charge: src.state.get_charge(),
				target_charge: src.state.target_charge(),
				recharge: src.state.recharge(),
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
						let id = swarm.rebuild(src_agent.id, &mut gen::Genome::new(dna), &timer);
						if let Some(agent) = swarm.get_mut(id) {
							for (src_segment, dest_segment) in src_agent.segments.iter().zip(agent.segments_mut().iter_mut()) {
								dest_segment.transform = geometry::Transform {
									position: geometry::Position::new(src_segment.x, src_segment.y),
									angle: src_segment.angle,
								};
								dest_segment.state.set_maturity(src_segment.maturity);
							}
						}
						registered.push(id);
					}
				}
			}
		}
		for id in registered {
			world.register(id);
		}
	}

	pub fn from_string(source: &str, dest: &mut world::World) {
		let result: Result<World, _> = serde_json::from_str(source);
		match result {
			Ok(src) => Self::from_world(src, dest),
			Err(_) => {}
		}
	}

	pub fn to_string(world: &world::World) -> String {
		let s_world = Self::to_world(world);
		serde_json::to_string_pretty(&s_world).unwrap()
	}

	pub fn save(world: &world::World) -> bool {
		let s_world = Self::to_world(world);
		serde_json::to_writer_pretty(file,&s_world).is_ok()
 	}
}
