use backend::world;
use serde_json;

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
	agent: Vec<Agent>,
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
		let swarms = Vec::new();

		let s_world = World {
			extent_min: Vector {
				x: 0.0,
				y: 0.0,
			},
			extent_max: Vector {
				x: 0.0,
				y: 0.0,
			},
			swarms,
			regenerations: 0,
			minion_gene_pool: Vec::new(),
			resource_gene_pool: Vec::new(),

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