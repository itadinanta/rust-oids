use backend::world;
use serde_json;
use serialize::base64::{self, ToBase64, FromBase64};

#[derive(Serialize, Deserialize, Debug)]
struct Vector {
	x: f32,
	y: f32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Transform {
	pub position: Vector,
	pub angle: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Segment {
	transform: Transform,
	motion: Transform,
	rest_angle: f32,
	index: usize,
	age_seconds: f64,
	age_frames: usize,
	maturity: f32,
	charge: f32,
	target_charge: f32,
	recharge: f32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Agent {
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
	agent_type: usize,
	agents: Vec<Agent>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct World {
	extent_min: Vector,
	extent_max: Vector,
	swarms: Vec<Swarm>,
	regenerations: usize,
	minion_gene_pool: Vec<String>,
	resource_gene_pool: Vec<String>,
}

pub struct Serializer;

impl Serializer {
	pub fn to_string(world: &world::World) -> String {
		fn serialize_agent(src: &world::agent::Agent) -> Agent {}

		fn serialize_swarm(src: &world::swarm::Swarm) -> Swarm {
			Swarm {
				agent_type: src.agent_type as usize,
				agents: src.agents().iter().map(|(k, v)| -> serialize_agent(v)),
			}
		}

		let swarms = Vec::new();
		let minion_gene_pool: Vec<_> = world.minion_gene_pool
			.gene_pool_iter()
			.map(|dna| dna.to_base64(base64::STANDARD))
			.collect();
		let resource_gene_pool: Vec<_> = world.resource_gene_pool
			.gene_pool_iter()
			.map(|dna| dna.to_base64(base64::STANDARD))
			.collect();
		let s_world = World {
			extent_min: Vector {
				x: world.extent.min.x,
				y: world.extent.min.y,
			},
			extent_max: Vector {
				x: world.extent.max.x,
				y: world.extent.max.y,
			},
			swarms,
			regenerations: world.regenerations,
			minion_gene_pool,
			resource_gene_pool,

		};
		let serialized = serde_json::to_string(&s_world).unwrap();
		serialized
	}

	pub fn from_string(source: &str, mut world: &world::World) {
		let result: Result<World, _> = serde_json::from_str(source);
		match result {
			Ok(world) => {}
			Err(_) => {}
		}
	}
}