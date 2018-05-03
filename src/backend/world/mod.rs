pub mod alert;
pub mod segment;
pub mod agent;
pub mod swarm;
pub mod gen;
pub mod phen;
pub mod particle;
pub mod persist;

use backend::obj;
use backend::obj::*;
use rand;
use chrono::Utc;
use chrono::DateTime;
use std::f32::consts;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io;
use std::io::Write;
use std::fs;

use app::constants::*;
use core::clock::*;
use core::geometry::*;
use core::geometry::Transform;
use core::resource::ResourceLoader;
use serialize::base64::{self, ToBase64};
use backend::messagebus::{Outbox, Message};
use self::agent::Agent;
use self::agent::AgentType;
use self::agent::TypedAgent;
use self::swarm::*;
use self::particle::Particle;

pub use self::alert::Alert;

pub trait AgentState {
	fn agent(&self, id: obj::Id) -> Option<&Agent>;
}

pub struct World {
	pub extent: Rect,
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
	fn agent(&self, id: obj::Id) -> Option<&Agent> {
		self.swarms.get(&id.type_of()).and_then(|m| m.get(id))
	}
}

#[derive(Clone)]
pub enum Emission {
	CW(Angle),
	CCW(Angle),
	Random,
}

#[derive(Clone)]
pub struct Feeder {
	transform: Transform,
	rate: Seconds,
	emission: Emission,
}

impl Feeder {
	pub fn new(x: f32, y: f32, rate: Seconds, emission: Emission) -> Self {
		Feeder {
			transform: Transform::from_position(Position::new(x, y)),
			rate,
			emission,
		}
	}
	pub fn rate(&self) -> Seconds {
		self.rate
	}
	pub fn emission(&self) -> Emission {
		self.emission.clone()
	}
}

impl Transformable for Feeder {
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
		where
			R: ResourceLoader<u8>, {
		let mut swarms = HashMap::new();
		let types = AgentType::all();
		let clock = SimulationTimer::new();
		for t in types {
			swarms.insert(*t, Swarm::new(*t, phen::phenotype_of(t)));
		}
		fn default_gene_pool(_: io::Error) -> gen::GenePool {
			gen::GenePool::parse_from_base64(DEFAULT_MINION_GENE_POOL)
		}
		let emitter_rate = Seconds::new(EMITTER_PERIOD);
		let num_emitters: usize = 7;
		let feeders = (0..num_emitters).map(|i| {
			let (s, c) = (consts::PI * 2. * (i as f32 / num_emitters as f32)).sin_cos();
			Feeder::new(c * EMITTER_DISTANCE, s * EMITTER_DISTANCE, emitter_rate, Emission::Random)
		}).collect::<Vec<_>>();
		World {
			extent: Rect::new(-WORLD_RADIUS, -WORLD_RADIUS, WORLD_RADIUS, WORLD_RADIUS),
			swarms,
			feeders,
			minion_gene_pool: res.load(minion_gene_pool)
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

	pub fn tick(&mut self, dt: Seconds) {
		self.clock.tick(dt);
	}

	pub fn seconds(&self) -> Seconds { self.clock.seconds() }

	pub fn extinctions(&self) -> usize {
		if self.regenerations > 1 { self.regenerations - 1 } else { 0usize }
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
			&clock);
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
			&clock);
		let livery_color = self.agent(id).unwrap()
			.segment(0).unwrap()
			.livery.albedo;
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
		let livery_color = self.agent(id).unwrap()
			.segment(0).unwrap()
			.livery.albedo;
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
		let livery_color = self.agent(id).unwrap()
			.segment(0).unwrap()
			.livery.albedo;
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

	pub fn spawn_player(&mut self, pos: Position, _motion: Motion) -> obj::Id {
		let mut gen = gen::Genome::copy_from(&[0, 0, 0, 0]);
		let clock = self.clock.clone();
		let id = self.swarm_mut(&AgentType::Player).spawn(
			&mut gen,
			agent::InitialState {
				transform: Transform::new(pos, 0.),
				charge: DEFAULT_MINION_CHARGE,
				..Default::default()
			},
			&clock,
		);
		self.register(id)
	}

	pub fn get_player_agent_id(&self) -> Option<obj::Id> {
		self.registered_player_id
	}

	fn get_player_segment(&mut self) -> Option<&mut segment::Segment> {
		self.registered_player_id.and_then(move |id|
			self.agent_mut(id).and_then(|player_agent|
				player_agent.segment_mut(0)
			)
		)
	}

	pub fn get_player_world_position(&self) -> Option<Position> {
		self.registered_player_id.and_then(move |id|
			self.agent(id).and_then(move |player_agent|
				player_agent.segment(0).map(move |segment|
					segment.transform.position
				)
			)
		)
	}

	pub fn primary_fire(&mut self, outbox: &Outbox, bullet_speed: f32) {
		self.get_player_segment().map(move |segment| {
			let angle = segment.transform.angle.clone();
			let scale = segment.growing_radius();
			let zero_dir = Position::unit_y();
			(Transform::new(segment.transform.apply(scale * zero_dir), angle),
			 Motion::new(segment.transform.apply_rotation(bullet_speed * zero_dir), 0.))
		})
			.map(|(t, v)| {
				outbox.post(Alert::NewBullet(0).into());
				self.new_resource(t, v.clone());
			});
	}

	pub fn set_player_intent(&mut self, intent: segment::Intent) {
		self.get_player_segment().map(|segment| {
			segment.state.intent = intent;
		});
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

	pub fn registered(&mut self) -> Box<[Id]> {
		self.registered
			.drain()
			.collect::<Vec<_>>()
			.into_boxed_slice()
	}

	#[allow(dead_code)]
	pub fn agent(&self, id: obj::Id) -> Option<&Agent> {
		self.swarms.get(&id.type_of()).and_then(|m| m.get(id))
	}

	pub fn for_all_agents<F>(&mut self, callback: &mut F)
		where
			F: FnMut(&mut Agent), {
		for (_, swarm) in self.swarms.iter_mut() {
			for (_, mut agent) in swarm.agents_mut().iter_mut() {
				callback(&mut agent)
			}
		}
	}

	pub fn agent_mut(&mut self, id: obj::Id) -> Option<&mut Agent> {
		self.swarms.get_mut(&id.type_of()).and_then(
			|m| m.get_mut(id),
		)
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

	pub fn feeders(&self) -> &[Feeder] {
		self.feeders.as_slice()
	}

	pub fn swarms(&self) -> &SwarmMap {
		&self.swarms
	}

	pub fn particles(&self) -> &[Particle] {
		&self.particles
	}

	pub fn clear_particles(&mut self) {
		self.particles.clear();
	}

	pub fn add_particle(&mut self, particle: Particle) {
		self.particles.push(particle);
	}

	pub fn cleanup_before(&mut self) {
		self.clear_particles();
	}

	pub fn sweep(&mut self) -> Box<[Agent]> {
		let mut v = Vec::new();
		for (_, agents) in self.swarms.iter_mut() {
			agents.free_resources(&mut v);
		}
		v.into_boxed_slice()
	}

	pub fn dump(&self) -> io::Result<String> {
		let now: DateTime<Utc> = Utc::now();
		//persist::Serializer::to_string(self).iter().for_each(|s| println!("{}", s));

		let file_name = now.format(DUMP_FILE_PATTERN_JSON).to_string();
		persist::Serializer::save(&file_name, self).is_ok();

		let file_name = now.format(DUMP_FILE_PATTERN_CSV).to_string();
		let mut f = fs::File::create(&file_name)?;
		for (_, agent) in self.agents(agent::AgentType::Minion).iter() {
			info!("{}", agent.dna().to_base64(base64::STANDARD));
			f.write_fmt(format_args!(
				"{}\n",
				agent.dna().to_base64(base64::STANDARD)
			))?;
		}
		Ok(file_name)
	}
}
