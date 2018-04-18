use backend::world;
use core::geometry;
use core::clock;
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
	minion_gene_pool_index: usize,
	resource_gene_pool: Vec<String>,
	resource_gene_pool_index: usize,
}

impl From<geometry::Position> for Vector {
	fn from(src: geometry::Position) -> Vector {
		Vector {
			x: src.x,
			y: src.y,
		}
	}
}

impl From<geometry::Transform> for Transform {
	fn from(src: geometry::Transform) -> Transform {
		Transform {
			position: src.position.into(),
			angle: src.angle,
		}
	}
}

impl From<geometry::Motion> for Transform {
	fn from(src: geometry::Motion) -> Transform {
		Transform {
			position: src.velocity.into(),
			angle: src.spin,
		}
	}
}

pub struct Serializer;

impl Serializer {
	pub fn to_string(world: &world::World) -> String {
		fn serialize_segment(src: &world::segment::Segment) -> Segment {
			let transform = src.transform.clone().into();
			let motion = src.motion.clone().unwrap_or_default().into();
			Segment {
				transform,
				motion,
				rest_angle: src.rest_angle,
				index: src.index as usize,
				age_seconds: src.state.age_seconds().into(),
				age_frames: src.state.age_frames(),
				maturity: src.state.maturity(),
				charge: src.state.get_charge(),
				target_charge: src.state.target_charge(),
				recharge: src.state.recharge(),
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

		fn serialize_swarm(src: &world::swarm::Swarm, timer: &clock::SimulationTimer) -> Swarm {
			Swarm {
				agent_type: src.agent_type() as usize,
				agents: src.agents().iter().map(|(k, v)| serialize_agent(v, timer)).collect(),
			}
		}

		let swarms = world.swarms()
			.iter()
			.map(|(k, v)| serialize_swarm(v, &world.clock))
			.collect();
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
			minion_gene_pool_index: world.minion_gene_pool.gene_pool_index(),
			resource_gene_pool,
			resource_gene_pool_index: world.resource_gene_pool.gene_pool_index(),
		};
		let serialized = serde_json::to_string_pretty(&s_world).unwrap();
		serialized
	}

	pub fn from_string(source: &str, world: &mut world::World) {
		let result: Result<World, _> = serde_json::from_str(source);
		match result {
			Ok(src) => {
				world.extent.min.x = src.extent_min.x;
				world.extent.min.y = src.extent_min.y;
				world.extent.max.x = src.extent_max.x;
				world.extent.max.y = src.extent_max.y;
				world.regenerations = src.regenerations;

				world.minion_gene_pool.populate_from_base64(&src.minion_gene_pool, src.minion_gene_pool_index);
				world.resource_gene_pool.populate_from_base64(&src.resource_gene_pool, src.resource_gene_pool_index);
			}
			Err(_) => {}
		}
	}
}