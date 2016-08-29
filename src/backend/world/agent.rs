use backend::obj::*;
use std::f32::consts;
use cgmath;
use cgmath::EuclideanVector;
use core::geometry::*;
use core::clock::*;
use backend::world::AgentType;
use backend::world::segment;
use backend::world::segment::Segment;
use num::FromPrimitive;

pub type Dna = Box<[u8]>;

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
	flags Flags: u32 {
		const DEAD       = 0x1,
		const ACTIVE     = 0x2,
	}
}

pub struct State {
	lifespan: Hourglass<SystemStopwatch>,
	flags: Flags,
	power: f32,
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

	pub fn consume(&mut self, q: f32) -> f32 {
		let residual = f32::min(self.power, q);
		self.power -= residual;
		residual
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

	pub fn segment_mut(&mut self, index: SegmentIndex) -> Option<&mut Segment> {
		self.segments.get_mut(index as usize)
	}

	pub fn brain(&self) -> Brain<f32> {
		self.brain.clone()
	}

	pub fn type_of(&self) -> AgentType {
		AgentType::from_usize(self.id & 0xff).unwrap_or(AgentType::Prop)
	}
}

pub struct AgentBuilder {
	id: Id,
	material: Material,
	livery: Livery,
	dna: Dna,
	state: segment::State,
	segments: Vec<Segment>,
}

impl AgentBuilder {
	pub fn new(id: Id, material: Material, livery: Livery, dna: &Dna, state: segment::State) -> Self {
		AgentBuilder {
			id: id,
			material: material,
			livery: livery,
			state: state,
			dna: dna.clone(),
			segments: Vec::new(),
		}
	}

	pub fn start(&mut self, position: Position, angle: f32, shape: &Shape) -> &mut Self {
		let segment = self.new_segment(shape,
		                               Winding::CW,
		                               position,
		                               angle,
		                               None,
		                               segment::TORSO | segment::MIDDLE);
		self.segments.clear();
		self.segments.push(segment);
		self
	}

	#[inline]
	pub fn add(&mut self,
	           parent_index: SegmentIndex,
	           attachment_index_offset: isize,
	           shape: &Shape,
	           flags: segment::Flags)
	           -> &mut Self {
		self.addw(parent_index,
		          attachment_index_offset,
		          shape,
		          Winding::CW,
		          flags | segment::MIDDLE)
	}
	#[inline]
	pub fn addl(&mut self,
	            parent_index: SegmentIndex,
	            attachment_index_offset: isize,
	            shape: &Shape,
	            flags: segment::Flags)
	            -> &mut Self {
		self.addw(parent_index,
		          attachment_index_offset,
		          shape,
		          Winding::CCW,
		          flags | segment::LEFT)
	}
	#[inline]
	pub fn addr(&mut self,
	            parent_index: SegmentIndex,
	            attachment_index_offset: isize,
	            shape: &Shape,
	            flags: segment::Flags)
	            -> &mut Self {
		self.addw(parent_index,
		          attachment_index_offset,
		          shape,
		          Winding::CW,
		          flags | segment::RIGHT)
	}

	pub fn addw(&mut self,
	            parent_index: SegmentIndex,
	            attachment_index_offset: isize,
	            shape: &Shape,
	            winding: Winding,
	            flags: segment::Flags)
	            -> &mut Self {
		let parent = self.segments[parent_index as usize].clone();//urgh!;
		let parent_pos = parent.transform.position;
		let parent_angle = parent.transform.angle;
		let parent_length = parent.mesh.shape.length() as isize;
		let attachment_index = ((attachment_index_offset + parent_length) % parent_length) as usize;
		let p0 = cgmath::Matrix2::from_angle(cgmath::rad(parent_angle)) * parent.mesh.vertices[attachment_index];
		let angle = f32::atan2(p0.y, p0.x);
		let r0 = p0.length() * parent.mesh.shape.radius();
		let r1 = shape.radius();
		let segment = self.new_segment(shape,
		                               winding,
		                               parent_pos + (p0 * (r0 + r1)),
		                               consts::PI / 2. + angle,
		                               parent.new_attachment(attachment_index as AttachmentIndex),
		                               flags);
		self.segments.push(segment);
		self
	}

	pub fn index(&self) -> SegmentIndex {
		match self.segments.len() {
			0 => 0,
			n => (n - 1) as SegmentIndex,
		}
	}

	fn new_segment(&mut self,
	               shape: &Shape,
	               winding: Winding,
	               position: Position,
	               angle: f32,
	               attachment: Option<segment::Attachment>,
	               flags: segment::Flags)
	               -> segment::Segment {
		segment::Segment {
			index: self.segments.len() as SegmentIndex,
			transform: Transform::new(position, angle),
			mesh: Mesh::from_shape(shape.clone(), winding),
			material: self.material.clone(),
			livery: self.livery.clone(),
			state: self.state.clone(),
			attached_to: attachment,
			flags: flags,
		}
	}

	pub fn build(&self) -> Agent {
		let order = self.segments.len() as f32;
		let d0 = 2. * order;

		Agent {
			id: self.id,
			state: State {
				flags: ACTIVE,
				lifespan: Hourglass::new(10.),
				power: 10.,
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
			dna: self.dna.clone(),
			segments: self.segments.clone().into_boxed_slice(),
		}
	}
}
