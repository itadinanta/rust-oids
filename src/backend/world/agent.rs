use std::collections::HashMap;
use std::fmt;
use std::f32;
use num::Float;
use num::FromPrimitive;
use core::geometry::*;
use core::geometry::Transform;
use core::clock::*;
use core::util;
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
		Key {
			agent_id: id,
			..Default::default()
		}
	}

	pub fn with_segment(id: obj::Id, segment_index: obj::SegmentIndex) -> Key {
		Key {
			agent_id: id,
			segment_index,
			..Default::default()
		}
	}

	pub fn with_bone(agent_id: obj::Id, segment_index: obj::SegmentIndex, bone_index: obj::BoneIndex) -> Key {
		Key { agent_id, segment_index, bone_index }
	}

	pub fn no_bone(&self) -> Key {
		Key {
			bone_index: 0,
			..*self
		}
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
const AGENT_TYPES: &'static [AgentType] = &[
	AgentType::Minion,
	AgentType::Spore,
	AgentType::Player,
	AgentType::FriendlyBullet,
	AgentType::Enemy,
	AgentType::EnemyBullet,
	AgentType::Resource,
	AgentType::Prop,
];

impl AgentType {
	pub fn all() -> &'static [AgentType] {
		AGENT_TYPES
	}
}

// for simplicity, inputs = intermediate = output
pub const N_WEIGHTS: usize = 4;

pub type InputVector<S> = [S; N_WEIGHTS];
pub type OutputVector<S> = [S; N_WEIGHTS];

pub type WeightVector<T> = [T; N_WEIGHTS];
pub type WeightMatrix<T> = [WeightVector<T>; N_WEIGHTS];

#[derive(Clone, Default, Debug)]
pub struct GBrain<T: Copy + Default> {
	pub hunger: T,
	pub haste: T,
	pub prudence: T,
	pub fear: T,
	pub rest: T,
	pub thrust: T,
	pub weights_in: WeightMatrix<T>,
	pub weights_hidden: WeightMatrix<T>,
	pub weights_out: WeightMatrix<T>,
}

pub trait TypedBrain {
	type Parameter: Float;
	type WeightVector;
	type WeightMatrix;
}

pub trait Personality<S>
	where
		S: Copy + Float,
{
	fn hunger(&self) -> S;
	fn haste(&self) -> S;
	fn prudence(&self) -> S;
	fn fear(&self) -> S;
	fn rest(&self) -> S;
	fn thrust(&self) -> S;
	fn response(&self, input: &InputVector<S>) -> OutputVector<S>;
}

pub trait Layer<S, T>
	where
		T: Copy,
		S: Float + From<T>,
{
	fn activation(x: S) -> S {
		x / (S::one() + x.abs())
	}

	fn layer(inputs: &[S], weights: &[WeightVector<T>]) -> OutputVector<S> {
		let mut outputs = [S::zero(); N_WEIGHTS];
		for i in 0..outputs.len() {
			for j in 0..inputs.len() {
				outputs[i] = outputs[i] + inputs[j] * weights[i][j].into();
			}
			outputs[i] = Self::activation(outputs[i])
		}
		outputs
	}
}

impl<S, T> Layer<S, T> for GBrain<T>
	where
		T: Copy + Default,
		S: Float + From<T>,
{}

impl<T, S> Personality<S> for GBrain<T>
	where
		T: Copy + Default,
		S: Copy + Float + From<T>,
{
	fn hunger(&self) -> S {
		self.hunger.into()
	}
	fn haste(&self) -> S {
		self.haste.into()
	}
	fn prudence(&self) -> S {
		self.prudence.into()
	}
	fn fear(&self) -> S {
		self.fear.into()
	}
	fn rest(&self) -> S {
		self.rest.into()
	}
	fn thrust(&self) -> S {
		self.thrust.into()
	}

	fn response(&self, input: &InputVector<S>) -> OutputVector<S> {
		let output_in = Self::layer(input, &self.weights_in);
		let output_hidden = Self::layer(&output_in, &self.weights_hidden);
		let output_out = Self::layer(&output_hidden, &self.weights_out);
		output_out
	}
}

impl<T> TypedBrain for GBrain<T>
	where
		T: Default + Copy + Float,
{
	type Parameter = T;
	type WeightVector = WeightVector<Self::Parameter>;
	type WeightMatrix = WeightMatrix<Self::Parameter>;
}

pub type Brain = GBrain<f32>;


bitflags! {
	pub struct Flags: u32 {
		const DEAD       = 0x1;
		const ACTIVE     = 0x2;
		const SELECTED   = 0x1000;
	}
}

#[derive(Clone, Debug)]
pub struct Limits {
	max_energy: f32,
}

#[derive(Clone, Debug)]
pub struct State {
	lifecycle: Hourglass<SimulationTimer>,
	flags: Flags,
	phase: f32,
	energy: f32,
	target: Option<Id>,
	target_position: Position,
	limits: Limits,
	foreign_dna: Option<Dna>,
	trajectory: util::History<Position>,
}

impl State {
	#[inline]
	pub fn lifecycle(&self) -> &Hourglass<SimulationTimer> {
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

	pub fn phase(&self) -> f32 {
		let age = self.lifecycle.elapsed();
		age.into()
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

	pub fn is_fertilised(&self) -> bool {
		self.foreign_dna.is_some()
	}

	pub fn fertilise(&mut self, dna: &Dna) {
		self.foreign_dna = Some(dna.clone());
	}

	pub fn foreign_dna(&self) -> &Option<Dna> {
		&self.foreign_dna
	}

	pub fn toggle_selection(&mut self) {
		self.flags ^= Flags::SELECTED;
	}
	#[allow(unused)]
	pub fn select(&mut self) {
		self.flags |= Flags::SELECTED;
	}

	pub fn deselect(&mut self) {
		self.flags -= Flags::SELECTED;
	}

	pub fn selected(&self) -> bool {
		self.flags.contains(Flags::SELECTED)
	}

	pub fn die(&mut self) {
		self.flags |= Flags::DEAD;
		self.flags -= Flags::ACTIVE;
	}

	#[inline]
	pub fn is_alive(&self) -> bool {
		!self.flags.contains(Flags::DEAD)
	}

	#[inline]
	pub fn is_active(&self) -> bool {
		self.flags.contains(Flags::ACTIVE)
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

	pub fn track_position(&mut self, position: &Position) {
		self.trajectory.push(position.clone())
	}

	pub fn trajectory(&self) -> Box<[Position]> {
		self.trajectory
			.into_iter()
			.collect::<Vec<_>>()
			.into_boxed_slice()
	}
}

pub struct Agent {
	id: Id,
	brain: Brain,
	dna: Dna,
	gender: u8,
	pub state: State,
	pub segments: Box<[Segment]>,
}

impl Identified for Agent {
	fn id(&self) -> Id {
		self.id
	}
}

impl Transformable for Agent {
	fn transform(&self) -> &Transform {
		self.segments.first().unwrap().transform()
	}
	fn transform_to(&mut self, t: &Transform) {
		self.segments.first_mut().unwrap().transform_to(t);
	}
}

impl Agent {
	#[inline]
	pub fn dna(&self) -> &Dna {
		&self.dna
	}

	#[inline]
	pub fn gender(&self) -> u8 {
		self.gender
	}

	#[inline]
	pub fn segments(&self) -> &[Segment] {
		&self.segments
	}

	#[inline]
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

	pub fn brain(&self) -> &Brain {
		&self.brain
	}

	pub fn first_segment(&self, flags: segment::Flags) -> Option<Segment> {
		self.segments
			.iter()
			.find(|segment| segment.flags.contains(flags))
			.map(|sensor| sensor.clone())
	}

	pub fn new(id: Id, gender: u8, brain: &Brain, dna: &Dna, segments: Box<[Segment]>, timer: SharedTimer<SimulationTimer>) -> Self {
		const SCALE: f32 = 100.;
		let max_energy = SCALE *
			segments
				.iter()
				.filter(|s| s.flags.contains(segment::Flags::STORAGE))
				.fold(0., |a, s| a + s.mesh.shape.radius().powi(2));
		Agent {
			id,
			state: State {
				flags: Flags::ACTIVE,
				lifecycle: Hourglass::new(timer, Seconds::new(5.)),
				energy: max_energy * 0.5,
				phase: 0.0f32,
				target: None,
				target_position: segments[0].transform.position,
				limits: Limits { max_energy },
				foreign_dna: None,
				trajectory: util::History::new(600),
			},
			brain: brain.clone(),
			gender,
			dna: dna.clone(),
			segments,
		}
	}
}

pub type AgentMap = HashMap<Id, Agent>;
