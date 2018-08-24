pub mod agent;
pub mod alert;
pub mod gen;
pub mod particle;
pub mod persist;
pub mod phen;
pub mod segment;
pub mod swarm;

use backend::obj;
use backend::obj::*;
use chrono::DateTime;
use chrono::Utc;
use rand;
use std::collections::HashMap;
use std::collections::HashSet;
use std::f32::consts;
use std::fs;
use std::io;
use std::io::Write;
use std::path;

use self::agent::Agent;
use self::agent::AgentType;
use self::agent::TypedAgent;
use self::particle::Particle;
use self::swarm::*;
use app::constants::*;
use backend::messagebus::{Message, Outbox};
use core::clock::*;
use core::color::Rgba;
use core::geometry::Transform;
use core::geometry::*;
use core::resource::ResourceLoader;
use serialize::base64::{self, ToBase64};

pub use self::alert::Alert;

pub trait AgentState {
	fn agent(&self, id: obj::Id) -> Option<&Agent>;
}

pub struct World {
	pub extent: Rect,
	phase: Rgba,
	swarms: HashMap<AgentType, Swarm>,
	feeders: Vec<Feeder>,
	registered: HashSet<Id>,
	registered_player_id: Option<Id>,
	regenerations: usize,
	minion_gene_pool: gen::GenePool,
	resource_gene_pool: gen::GenePool,
	clock: SimulationTimer,
	particles: Vec<Particle>,
}

impl AgentState for World {
	fn agent(&self, id: obj::Id) -> Option<&Agent> { self.swarms.get(&id.type_of()).and_then(|m| m.get(id)) }
}

#[derive(Clone)]
pub struct Feeder {
	transform: Transform,
	rate: Seconds,
	intensity: f32,
}

impl Feeder {
	pub fn new(x: f32, y: f32, rate: Seconds) -> Self {
		Feeder {
			transform: Transform::from_position(Position::new(x, y)),
			rate,
			intensity: 1.0,
		}
	}
	pub fn rate(&self) -> Seconds { self.rate }
	pub fn intensity(&self) -> f32 { self.intensity }
	pub fn set_intensity(&mut self, intensity: f32) { self.intensity = intensity }
}

impl Transformable for Feeder {
	fn transform(&self) -> &Transform { &self.transform }
	fn transform_to(&mut self, t: Transform) { self.transform = t; }
}

impl World {
	pub fn new<R>(res: &R, minion_gene_pool: &str) -> Self
	where R: ResourceLoader<u8> {
		let mut swarms = HashMap::new();
		let types = AgentType::all();
		let clock = SimulationTimer::new();
		for t in types {
			swarms.insert(*t, Swarm::new(*t, phen::phenotype_of(*t)));
		}
		fn default_gene_pool(_: io::Error) -> gen::GenePool {
			gen::GenePool::parse_from_base64(DEFAULT_MINION_GENE_POOL)
		}
		let emitter_rate = Seconds::new(EMITTER_PERIOD);
		let num_emitters: usize = 7;
		let feeders = (0..num_emitters)
			.map(|i| {
				let (s, c) = (consts::PI * 2. * (i as f32 / num_emitters as f32)).sin_cos();
				Feeder::new(c * EMITTER_DISTANCE, s * EMITTER_DISTANCE, emitter_rate)
			}).collect::<Vec<_>>();
		World {
			extent: Rect::new(-WORLD_RADIUS, -WORLD_RADIUS, WORLD_RADIUS, WORLD_RADIUS),
			phase: COLOR_TRANSPARENT,
			swarms,
			feeders,
			minion_gene_pool: res
				.load(minion_gene_pool)
				.map(|data| gen::GenePool::parse_from_resource(&data))
				.unwrap_or_else(default_gene_pool),
			resource_gene_pool: gen::GenePool::parse_from_base64(DEFAULT_RESOURCE_GENE_POOL),
			registered: HashSet::new(),
			registered_player_id: None,
			regenerations: 0usize,
			clock,
			particles: Vec::with_capacity(10000),
		}
	}

	pub fn clear(&mut self) {
		for (_, swarm) in self.swarms.iter_mut() {
			swarm.clear();
		}
		self.registered.clear();
		self.registered_player_id = None;
		self.particles.clear();
	}

	pub fn tick(&mut self, dt: Seconds) { self.clock.tick(dt); }

	pub fn seconds(&self) -> Seconds { self.clock.seconds() }

	pub fn extinctions(&self) -> usize {
		if self.regenerations > 1 {
			self.regenerations - 1
		} else {
			0usize
		}
	}

	pub fn new_resource(&mut self, transform: Transform, motion: Motion) -> obj::Id {
		let mut gen = &mut self.resource_gene_pool.next();
		let clock = self.clock.clone();
		let id = self.swarm_mut(&AgentType::Resource).spawn(
			&mut gen,
			agent::InitialState {
				transform,
				motion,
				charge: DEFAULT_RESOURCE_CHARGE,
				..Default::default()
			},
			&clock,
		);
		self.register(id)
	}

	pub fn decay_to_resource(&mut self, outbox: &Outbox, transform: Transform, dna: &gen::Dna) -> obj::Id {
		let clock = self.clock.clone();
		let id = self.swarm_mut(&AgentType::Resource).spawn(
			&mut gen::Genome::copy_from(dna),
			agent::InitialState {
				transform: transform.clone(),
				charge: DEFAULT_RESOURCE_CHARGE,
				..Default::default()
			},
			&clock,
		);
		let livery_color = self.agent(id).unwrap().segment(0).unwrap().livery.albedo;
		outbox.post(Message::NewEmitter(particle::Emitter::for_dead_minion(
			transform,
			livery_color,
		)));
		self.register(id)
	}

	pub fn new_spore(&mut self, outbox: &Outbox, transform: Transform, dna: &gen::Dna) -> obj::Id {
		let clock = self.clock.clone();
		let id = self.swarm_mut(&AgentType::Spore).spawn(
			&mut gen::Genome::copy_from(dna).mutate(&mut rand::thread_rng()),
			agent::InitialState {
				transform: transform.clone(),
				charge: DEFAULT_SPORE_CHARGE,
				..Default::default()
			},
			&clock,
		);
		let livery_color = self.agent(id).unwrap().segment(0).unwrap().livery.albedo;
		outbox.post(Message::NewEmitter(particle::Emitter::for_new_spore(
			transform,
			livery_color,
			id,
		)));
		self.register(id)
	}

	pub fn hatch_spore(&mut self, outbox: &Outbox, transform: Transform, dna: &gen::Dna) -> obj::Id {
		let clock = self.clock.clone();
		let id = self.swarm_mut(&AgentType::Minion).spawn(
			&mut gen::Genome::copy_from(dna),
			agent::InitialState {
				transform: transform.clone(),
				charge: DEFAULT_MINION_CHARGE,
				..Default::default()
			},
			&clock,
		);
		let livery_color = self.agent(id).unwrap().segment(0).unwrap().livery.albedo;
		outbox.post(Message::NewEmitter(particle::Emitter::for_new_minion(
			transform,
			livery_color,
		)));
		self.register(id)
	}

	pub fn randomize_minion(&mut self, pos: Position, motion: Motion) -> obj::Id {
		self.minion_gene_pool.randomize();
		self.new_minion(pos, motion)
	}

	pub fn init_minions(&mut self) {
		self.regenerations += 1;
		let n = self.minion_gene_pool.len();
		let clock = self.clock.clone();
		let mut r = self.extent.top_right().x * INITIAL_SPAWN_RADIUS_RATIO;
		let mut angle = 0.0f32;
		let angle_delta = consts::PI * 2. / INITIAL_SPAWN_RADIUS_SLICES as f32;
		for _ in 0..n {
			let pos = Position::new(r * angle.cos(), r * angle.sin());
			let mut gen = self.minion_gene_pool.next();
			let id = self.swarm_mut(&AgentType::Minion).spawn(
				&mut gen,
				agent::InitialState {
					transform: Transform::new(pos, angle + consts::PI / 2.),
					charge: DEFAULT_MINION_CHARGE,
					..Default::default()
				},
				&clock,
			);
			self.register(id);
			angle += angle_delta;
			r += INITIAL_SPAWN_RADIUS_INCREMENT;
		}
	}

	pub fn init_players(&mut self) {
		self.registered_player_id = Some(self.spawn_player(Position::new(0., 0.), Motion::default()))
	}

	pub fn spawn_player(&mut self, pos: Position, motion: Motion) -> obj::Id {
		let mut gen = gen::Genome::copy_from(&[0, 0, 0, 0]);
		let clock = self.clock.clone();
		let id = self.swarm_mut(&AgentType::Player).spawn(
			&mut gen,
			agent::InitialState {
				transform: Transform::new(pos, 0.),
				motion,
				charge: DEFAULT_MINION_CHARGE,
				..Default::default()
			},
			&clock,
		);
		self.register(id)
	}

	pub fn get_player_agent_id(&self) -> Option<obj::Id> { self.registered_player_id }

	fn get_player_segment_mut(&mut self) -> Option<&mut segment::Segment> {
		self.registered_player_id
			.and_then(move |id| self.agent_mut(id).and_then(|player_agent| player_agent.segment_mut(0)))
	}

	pub fn get_player_segment(&self) -> Option<&segment::Segment> {
		self.registered_player_id
			.and_then(move |id| self.agent(id).and_then(|player_agent| player_agent.segment(0)))
	}
	/*
		pub fn get_player_world_position(&self) -> Option<Position> {
			self.registered_player_id.and_then(move |id|
				self.agent(id).and_then(move |player_agent|
					player_agent.segment(0).map(move |segment|
						segment.transform.position
					)
				)
			)
		}
	*/
	pub fn primary_fire(&mut self, outbox: &Outbox, bullet_speed: f32) {
		let vectors = self.get_player_segment().map(move |segment| {
			let angle = segment.transform.angle;
			let scale = segment.growing_radius() + 0.5;
			let forward_dir = Position::unit_y();
			let transform = Transform::new(segment.transform.apply(scale * forward_dir), angle);
			let motion = Motion::new(
				segment.motion.velocity + segment.transform.apply_rotation(bullet_speed * forward_dir),
				0.,
			);
			(transform, motion)
		});
		if let Some((transform, motion)) = vectors {
			outbox.post(Alert::NewBullet(0).into());
			self.new_resource(transform, motion);
		}
	}

	pub fn set_player_intent(&mut self, intent: segment::Intent) {
		if let Some(segment) = self.get_player_segment_mut() {
			segment.state.intent = intent;
		}
	}

	pub fn new_minion(&mut self, pos: Position, motion: Motion) -> obj::Id {
		let angle = consts::PI / 2. + f32::atan2(pos.y, pos.x);
		let mut gen = self.minion_gene_pool.next();
		let clock = self.clock.clone();
		let id = self.swarm_mut(&AgentType::Minion).spawn(
			&mut gen,
			agent::InitialState {
				transform: Transform::new(pos, angle),
				motion,
				charge: 0.3,
				..Default::default()
			},
			&clock,
		);
		self.register(id)
	}

	pub fn register(&mut self, id: obj::Id) -> obj::Id {
		self.registered.insert(id);
		id
	}

	pub fn registered(&mut self) -> Box<[Id]> { self.registered.drain().collect::<Vec<_>>().into_boxed_slice() }

	#[allow(dead_code)]
	pub fn agent(&self, id: obj::Id) -> Option<&Agent> { self.swarms.get(&id.type_of()).and_then(|m| m.get(id)) }

	pub fn for_all_agents<F>(&mut self, callback: &mut F)
	where F: FnMut(&mut Agent) {
		for swarm in self.swarms.values_mut() {
			for agent in swarm.agents_mut().values_mut() {
				callback(agent)
			}
		}
	}

	pub fn agent_mut(&mut self, id: obj::Id) -> Option<&mut Agent> {
		self.swarms.get_mut(&id.type_of()).and_then(|m| m.get_mut(id))
	}

	pub fn agents(&self, agent_type: AgentType) -> &agent::AgentMap { self.swarms[&agent_type].agents() }

	pub fn agents_mut(&mut self, agent_type: AgentType) -> &mut agent::AgentMap {
		self.swarms.get_mut(&agent_type).unwrap().agents_mut()
	}

	pub fn swarm_mut(&mut self, agent_type: &AgentType) -> &mut Swarm { self.swarms.get_mut(&agent_type).unwrap() }

	pub fn feeders(&self) -> &[Feeder] { self.feeders.as_slice() }

	pub fn feeders_mut(&mut self) -> &mut [Feeder] { self.feeders.as_mut_slice() }

	pub fn swarms(&self) -> &SwarmMap { &self.swarms }

	pub fn phase(&self) -> Rgba { self.phase }

	pub fn phase_mut(&mut self) -> &mut Rgba { &mut self.phase }

	pub fn particles(&self) -> &[Particle] { &self.particles }

	pub fn clear_particles(&mut self) { self.particles.clear(); }

	pub fn add_particle(&mut self, particle: Particle) { self.particles.push(particle); }

	pub fn cleanup_before(&mut self) { self.clear_particles(); }

	pub fn sweep(&mut self) -> Box<[Agent]> {
		let mut v = Vec::new();
		for (_, agents) in self.swarms.iter_mut() {
			agents.free_resources(&mut v);
		}
		v.into_boxed_slice()
	}

	pub fn serialize(&self, containing_dir: &path::Path) -> io::Result<path::PathBuf> {
		let now: DateTime<Utc> = Utc::now();
		fs::create_dir_all(containing_dir).is_ok();
		let file_name = containing_dir.join(now.format(DUMP_FILE_PATTERN_JSON).to_string());
		persist::Serializer::save(file_name.as_path(), self)?;
		Ok(file_name)
	}

	pub fn dump(&self, containing_dir: &path::Path) -> io::Result<path::PathBuf> {
		let now: DateTime<Utc> = Utc::now();
		let file_name = containing_dir.join(now.format(DUMP_FILE_PATTERN_CSV).to_string());
		fs::create_dir_all(containing_dir).is_ok();
		let mut f = fs::File::create(&file_name)?;
		for (_, agent) in self.agents(agent::AgentType::Minion).iter() {
			info!("{}", agent.dna().to_base64(base64::STANDARD));
			f.write_fmt(format_args!("{}\n", agent.dna().to_base64(base64::STANDARD)))?;
		}
		Ok(file_name)
	}
}
