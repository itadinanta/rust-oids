use std::collections::HashMap;
use std::fmt;
use num::FromPrimitive;
use core::geometry::*;
use core::clock::*;
use backend::obj;
use backend::obj::*;
use backend::world::gen::Dna;
use backend::world::segment;
use backend::world::segment::Segment;

#[repr(packed)]
#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
pub struct Key {
	pub agent_id: obj::Id,
	pub segment_index: obj::SegmentIndex,
	pub bone_index: obj::BoneIndex,
}

impl Identified for Key {
	fn id(&self) -> obj::Id {
		self.agent_id
	}
}

impl Default for Key {
	fn default() -> Key {
		Key {
			agent_id: 0xdeadbeef,
			segment_index: 0,
			bone_index: 0,
		}
	}
}

impl Key {
	pub fn with_id(id: obj::Id) -> Key {
		Key { agent_id: id, ..Default::default() }
	}

	pub fn with_segment(id: obj::Id, segment_index: obj::SegmentIndex) -> Key {
		Key {
			agent_id: id,
			segment_index: segment_index,
			..Default::default()
		}
	}

	pub fn with_bone(id: obj::Id, segment_index: obj::SegmentIndex, bone_index: obj::BoneIndex) -> Key {
		Key {
			agent_id: id,
			segment_index: segment_index,
			bone_index: bone_index,
		}
	}

	pub fn no_bone(&self) -> Key {
		Key { bone_index: 0, ..*self }
	}
}

pub trait TypedAgent {
	fn type_of(&self) -> AgentType;
}

impl TypedAgent for Id {
	fn type_of(&self) -> AgentType {
		AgentType::from_usize(*self & 0xff).unwrap_or(AgentType::Prop)
	}
}

enum_from_primitive! {
	#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
	pub enum AgentType {
		Minion ,
		Spore,
		Player,
		FriendlyBullet,
		Enemy,
		EnemyBullet,
		Resource,
		Prop,
	}
}

// TODO: sure there must be a better way?
impl fmt::Display for AgentType {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let text = match self {
			&AgentType::Minion => "Minion",
			&AgentType::Spore => "Spore",
			&AgentType::Player => "Player",
			&AgentType::FriendlyBullet => "FriendlyBullet",
			&AgentType::Enemy => "Enemy",
			&AgentType::EnemyBullet => "EnemyBullet",
			&AgentType::Resource => "Resource",
			&AgentType::Prop => "Prop",
		};
		f.write_str(text)
	}
}

// TODO: is there a better way to derive this?
const AGENT_TYPES: &'static [AgentType] = &[AgentType::Minion,
                                            AgentType::Spore,
                                            AgentType::Player,
                                            AgentType::FriendlyBullet,
                                            AgentType::Enemy,
                                            AgentType::EnemyBullet,
                                            AgentType::Resource,
                                            AgentType::Prop];
impl AgentType {
	pub fn all() -> &'static [AgentType] {
		AGENT_TYPES
	}
}

#[derive(Clone)]
pub struct Brain<T> {
	pub timidity: T,
	pub caution: T,
	pub curiosity: T,
	pub hunger: T,
	pub focus: T,
	pub haste: T,
	pub fear: T,
	pub prudence: T,
	pub rest: T,
	pub thrust: T,
}

bitflags! {
	pub flags Flags: u32 {
		const DEAD       = 0x1,
		const ACTIVE     = 0x2,
	}
}

#[derive(Clone,Debug)]
pub struct Limits {
	max_energy: f32,
}

#[derive(Clone,Debug)]
pub struct State {
	lifecycle: Hourglass<SystemStopwatch>,
	flags: Flags,
	energy: f32,
	target: Option<Id>,
	target_position: Position,
	limits: Limits,
}

impl State {
	#[inline]
	pub fn lifecycle(&self) -> &Hourglass<SystemStopwatch> {
		&self.lifecycle
	}

	pub fn renew(&mut self) {
		self.lifecycle.renew()
	}

	pub fn energy(&self) -> f32 {
		self.energy
	}

	pub fn energy_ratio(&self) -> f32 {
		self.energy / self.limits.max_energy
	}

	pub fn consume(&mut self, q: f32) -> bool {
		if self.energy >= q {
			self.energy -= q;
			true
		} else {
			false
		}
	}

	pub fn consume_ratio(&mut self, ratio: f32) -> bool {
		let max = self.limits.max_energy;
		self.consume(max * ratio)
	}


	pub fn absorb(&mut self, q: f32) {
		self.energy = self.limits.max_energy.min(self.energy + q);
	}

	pub fn die(&mut self) {
		self.flags |= DEAD;
		self.flags -= ACTIVE;
	}

	#[inline]
	pub fn is_alive(&self) -> bool {
		!self.flags.contains(DEAD)
	}

	#[inline]
	pub fn is_active(&self) -> bool {
		self.flags.contains(ACTIVE)
	}

	pub fn target_position(&self) -> &Position {
		&self.target_position
	}

	pub fn target(&self) -> &Option<Id> {
		&self.target
	}

	pub fn retarget(&mut self, target: Option<Id>, position: Position) {
		self.target = target;
		self.target_position = position;
	}
}

pub struct Agent {
	id: Id,
	brain: Brain<f32>,
	dna: Dna,
	pub state: State,
	pub segments: Box<[Segment]>,
}

impl Identified for Agent {
	fn id(&self) -> Id {
		self.id
	}
}

impl Transformable for Agent {
	fn transform(&self) -> Transform {
		self.segments.first().unwrap().transform()
	}
	fn transform_to(&mut self, t: Transform) {
		self.segments.first_mut().unwrap().transform_to(t);
	}
}

impl Agent {
	pub fn dna(&self) -> &Dna {
		&self.dna
	}

	pub fn segments(&self) -> &[Segment] {
		&self.segments
	}

	pub fn segments_mut(&mut self) -> &mut [Segment] {
		&mut self.segments
	}

	pub fn segment(&self, index: SegmentIndex) -> Option<&Segment> {
		self.segments.get(index as usize)
	}

	pub fn last_segment(&self) -> &Segment {
		self.segments.last().unwrap()
	}

	pub fn segment_mut(&mut self, index: SegmentIndex) -> Option<&mut Segment> {
		self.segments.get_mut(index as usize)
	}

	pub fn brain(&self) -> &Brain<f32> {
		&self.brain
	}

	pub fn first_segment(&self, flags: segment::Flags) -> Option<Segment> {
		self.segments
			.iter()
			.find(|segment| segment.flags.contains(flags))
			.map(|sensor| sensor.clone())
	}

	pub fn new(id: Id, dna: &Dna, segments: Box<[Segment]>) -> Self {
		let max_energy = 100. *
		                 segments.iter()
			.filter(|s| s.flags.contains(segment::STORAGE))
			.fold(0., |a, s| a + s.mesh.shape.radius().powi(2));
		let order = segments.len() as f32;
		let d0 = 2. * order;

		Agent {
			id: id,
			state: State {
				flags: ACTIVE,
				lifecycle: Hourglass::new(5.),
				energy: max_energy * 0.5,
				target: None,
				target_position: segments[0].transform.position,
				limits: Limits { max_energy: max_energy },
			},
			brain: Brain {
				timidity: 2. * (12.0 - order),
				hunger: 4. * order,
				haste: 2. * order,
				prudence: 3. * order,

				caution: d0 * 2.0,
				focus: d0 * 1.5,
				curiosity: d0 * 1.2,
				fear: d0 * 0.01,

				rest: 0.1,
				thrust: 0.5,
			},
			dna: dna.clone(),
			segments: segments,
		}
	}
}

pub type AgentMap = HashMap<Id, Agent>;
