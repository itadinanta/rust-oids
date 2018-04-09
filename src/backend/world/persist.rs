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
	rest_angle: f32,
	motion: Transform,
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
