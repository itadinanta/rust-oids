use std::collections::HashMap;
use std::fmt;
use cgmath::Vector;
use num::FromPrimitive;
use core::geometry::*;
use core::clock::*;
use backend::obj;
use backend::obj::*;
use backend::world::gen::Dna;
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

impl fmt::Display for AgentType {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			&AgentType::Minion => f.write_str("Minion"),
			&AgentType::Spore => f.write_str("Spore"),
			&AgentType::Player => f.write_str("Player"),
			&AgentType::FriendlyBullet => f.write_str("FriendlyBullet"),
			&AgentType::Enemy => f.write_str("Enemy"),
			&AgentType::EnemyBullet => f.write_str("EnemyBullet"),
			&AgentType::Resource => f.write_str("Resource"),
			&AgentType::Prop => f.write_str("Prop"),
		}
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
pub struct State {
	lifespan: Hourglass<SystemStopwatch>,
	flags: Flags,
	power: f32,
	target: Option<Id>,
	target_position: Position,
}

impl State {
	#[inline]
	pub fn lifespan(&self) -> &Hourglass<SystemStopwatch> {
		&self.lifespan
	}

	pub fn renew(&mut self) {
		self.lifespan.renew()
	}

	pub fn power(&self) -> f32 {
		self.power
	}

	pub fn consume(&mut self, q: f32) -> bool {
		if self.power >= q {
			self.power -= q;
			true
		} else {
			false
		}
	}

	pub fn absorb(&mut self, q: f32) {
		self.power += q;
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
}

pub struct Agent {
	id: Id,
	brain: Brain<f32>,
	pub state: State,
	dna: Dna,
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

	pub fn brain(&self) -> Brain<f32> {
		self.brain.clone()
	}

	pub fn new(id: Id, d0: f32, order: f32, dna: &Dna, segments: Box<[Segment]>) -> Self {
		Agent {
			id: id,
			state: State {
				flags: ACTIVE,
				lifespan: Hourglass::new(5.),
				power: 3. * order,
				target: None,
				target_position: Position::zero(),
			},
			brain: Brain {
				timidity: 2. * (12.0 - order),
				hunger: 4. * order,
				haste: 2. * order,
				prudence: 3. * order,

				caution: d0 * 2.0,
				focus: d0 * 1.5,
				curiosity: d0 * 1.2,
				fear: d0 * 0.5,

				rest: 0.1,
				thrust: 0.5,
			},
			dna: dna.clone(),
			segments: segments,
		}
	}
}

pub type AgentMap = HashMap<Id, Agent>;
