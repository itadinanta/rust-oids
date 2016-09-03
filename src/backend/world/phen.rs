use backend::obj::*;
use std::f32::consts;
use core::color;
use core::color::ToRgb;
use core::geometry::*;
use backend::world::segment;
use backend::world::segment::*;
use backend::world::agent;
use backend::world::agent::Agent;
use backend::world::gen::*;
use cgmath;
use cgmath::EuclideanVector;

pub trait Phenotype {
	fn develop(gen: &mut Genome, id: Id, transform: Transform, motion: Option<Motion>, charge: f32) -> agent::Agent;
}

pub struct Resource {}
pub struct Minion {}
pub struct Spore {}

impl Phenotype for Resource {
	fn develop(gen: &mut Genome, id: Id, transform: Transform, motion: Option<Motion>, charge: f32) -> agent::Agent {

		let albedo = color::YPbPr::new(0.5, gen.next_float(-0.5, 0.5), gen.next_float(-0.5, 0.5));
		let body = gen.eq_triangle();
		let mut builder = AgentBuilder::new(id,
		                                    Material { density: 1.0, ..Default::default() },
		                                    Livery { albedo: albedo.to_rgba(), ..Default::default() },
		                                    gen.dna(),
		                                    segment::State::with_charge(charge, 0., charge));
		builder.start(Transform::from_position(transform.position), motion, &body).build()
	}
}

impl Phenotype for Minion {
	fn develop(gen: &mut Genome, id: Id, transform: Transform, motion: Option<Motion>, charge: f32) -> agent::Agent {
		let albedo = color::Hsl::new(gen.next_float(0., 1.), 0.5, 0.5);
		let mut builder = AgentBuilder::new(id,
		                                    Material { density: 0.2, ..Default::default() },
		                                    Livery { albedo: albedo.to_rgba(), ..Default::default() },
		                                    gen.dna(),
		                                    segment::State::with_charge(0., charge, charge));
		let torso_shape = gen.npoly(5, true);
		let torso = builder.start(transform, motion, &torso_shape).index();
		let arm_shape = gen.star();
		let leg_shape = gen.star();
		let head_shape = gen.iso_triangle();
		let antenna_shape = gen.triangle();
		let tail_shape = gen.vbar();
		builder.addr(torso, 2, &arm_shape, ARM | JOINT | ACTUATOR | RUDDER)
			.addl(torso, -2, &arm_shape, ARM | JOINT | ACTUATOR | RUDDER);

		let head = builder.add(torso, 0, &head_shape, HEAD | MOUTH | SENSOR).index();
		builder.addr(head, 1, &antenna_shape, HEAD | MOUTH | ACTUATOR | RUDDER)
			.addl(head, 2, &antenna_shape, HEAD | MOUTH | ACTUATOR | RUDDER);

		let mut belly = torso;
		let mut belly_mid = torso_shape.mid();
		for _ in 0..gen.next_integer(0, 4) {
			let belly_shape = gen.poly(true);

			belly = builder.add(belly, belly_mid, &belly_shape, BELLY | JOINT).index();
			belly_mid = belly_shape.mid();
			if gen.next_integer(0, 4) == 0 {
				builder.addr(belly, 2, &arm_shape, ARM | ACTUATOR | RUDDER)
					.addl(belly, -2, &arm_shape, ARM | ACTUATOR | RUDDER);
			}
		}

		builder.addr(belly, belly_mid - 1, &leg_shape, LEG | ACTUATOR | THRUSTER)
			.addl(belly,
			      -(belly_mid - 1),
			      &leg_shape,
			      LEG | ACTUATOR | THRUSTER)
			.add(belly, belly_mid, &tail_shape, TAIL | ACTUATOR | BRAKE)
			.build()
	}
}

impl Phenotype for Spore {
	fn develop(gen: &mut Genome, id: Id, transform: Transform, motion: Option<Motion>, charge: f32) -> agent::Agent {
		let albedo = color::Hsl::new(gen.next_float(0., 1.), 0.5, 0.5);
		let mut builder = AgentBuilder::new(id,
		                                    Material { density: 0.5, ..Default::default() },
		                                    Livery { albedo: albedo.to_rgba(), ..Default::default() },
		                                    gen.dna(),
		                                    segment::State::with_charge(0., charge, charge));
		builder.start(transform, motion, &gen.ball()).build()
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

	pub fn start(&mut self, transform: Transform, initial_vel: Option<Motion>, shape: &Shape) -> &mut Self {
		let segment = self.new_segment(shape,
		                               Winding::CW,
		                               transform,
		                               initial_vel,
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
		                               Transform::new(parent_pos + (p0 * (r0 + r1)), consts::PI / 2. + angle),
		                               None,
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
	               transform: Transform,
	               motion: Option<Motion>,
	               attachment: Option<segment::Attachment>,
	               flags: segment::Flags)
	               -> segment::Segment {
		segment::Segment {
			index: self.segments.len() as SegmentIndex,
			transform: transform,
			motion: motion,
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

		Agent::new(self.id,
		           d0,
		           order,
		           &self.dna,
		           self.segments.clone().into_boxed_slice())
	}
}
