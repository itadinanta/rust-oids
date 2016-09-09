use backend::obj::*;
use std::f32::consts;
use core::color;
use core::color::ToRgb;
use core::geometry::*;
use backend::world::segment;
use backend::world::segment::*;
use backend::world::agent;
use backend::world::agent::Agent;
use backend::world::agent::GBrain;
use backend::world::gen::*;
use cgmath;
use cgmath::EuclideanVector;

pub trait Phenotype {
	fn develop(gen: &mut Genome, id: Id, transform: &Transform, motion: Option<&Motion>, charge: f32) -> agent::Agent;
}

pub struct Resource {}
pub struct Minion {}
pub struct Spore {}

impl Phenotype for Resource {
	fn develop(gen: &mut Genome, id: Id, transform: &Transform, motion: Option<&Motion>, charge: f32) -> agent::Agent {
		gen.next_integer::<u8>(0, 3);
		let albedo = color::YPbPr::new(0.5, gen.next_float(-0.5, 0.5), gen.next_float(-0.5, 0.5));
		let body = gen.eq_triangle();
		let mut builder = AgentBuilder::new(id,
		                                    Material { density: 1.0, ..Default::default() },
		                                    Livery { albedo: albedo.to_rgba(), ..Default::default() },
		                                    gen.dna(),
		                                    segment::State::with_charge(charge, 0., charge));
		builder.start(transform, motion, &body).build()
	}
}

impl Phenotype for Minion {
	fn develop(gen: &mut Genome, id: Id, transform: &Transform, motion: Option<&Motion>, charge: f32) -> agent::Agent {
		let gender = gen.next_integer::<u8>(0, 3);
		let tint = gen.next_float(0., 1.);
		let albedo = color::Hsl::new(tint, 0.5, 0.5);
		let mut builder = AgentBuilder::new(id,
		                                    Material { density: 0.2, ..Default::default() },
		                                    Livery { albedo: albedo.to_rgba(), ..Default::default() },
		                                    gen.dna(),
		                                    segment::State::with_charge(0., charge, charge));
		builder.gender(gender);
		let torso_shape = gen.any_poly();
		let torso = builder.start(transform, motion, &torso_shape).index();
		let arm_shape = gen.star();
		let leg_shape = gen.star();
		let head_shape = gen.iso_triangle();
		let antenna_shape = gen.triangle();
		let tail_shape = gen.vbar();
		let i = ::std::cmp::max((torso_shape.length() as isize / 5), 1);
		builder.addr(torso, i, &arm_shape, ARM | JOINT | ACTUATOR | RUDDER)
			.addl(torso, -i, &arm_shape, ARM | JOINT | ACTUATOR | RUDDER);

		let head = builder.add(torso, 0, &head_shape, HEAD | SENSOR).index();
		builder.addr(head, 1, &antenna_shape, HEAD | MOUTH | ACTUATOR | RUDDER)
			.addl(head, 2, &antenna_shape, HEAD | MOUTH | ACTUATOR | RUDDER);

		let mut belly = torso;
		let mut belly_mid = torso_shape.mid();
		while gen.next_integer(0, 4) == 0 {
			let belly_shape = gen.poly(true);

			belly = builder.add(belly, belly_mid, &belly_shape, STORAGE | JOINT).index();
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
	fn develop(gen: &mut Genome, id: Id, transform: &Transform, motion: Option<&Motion>, charge: f32) -> agent::Agent {
		let gender = gen.next_integer::<u8>(0, 3);
		let tint = gen.next_float(0., 1.);
		let albedo = color::Hsl::new(tint, 0.5, 0.5);

		let mut builder = AgentBuilder::new(id,
		                                    Material { density: 0.5, ..Default::default() },
		                                    Livery { albedo: albedo.to_rgba(), ..Default::default() },
		                                    gen.dna(),
		                                    segment::State::with_charge(0., charge, charge));
		builder.gender(gender).start(transform, motion, &gen.ball()).build()
	}
}

pub struct AgentBuilder {
	id: Id,
	material: Material,
	livery: Livery,
	gender: u8,
	brain: GBrain<i8>,
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
			gender: 0u8,
			brain: GBrain::default(),
			dna: dna.clone(),
			segments: Vec::new(),
		}
	}

	pub fn start(&mut self, transform: &Transform, motion: Option<&Motion>, shape: &Shape) -> &mut Self {
		let segment = self.new_segment(shape,
		                               Winding::CW,
		                               transform,
		                               motion,
		                               None,
		                               segment::CORE | segment::STORAGE | segment::MIDDLE);
		self.segments.clear();
		self.segments.push(segment);
		self
	}

	#[inline]
	pub fn gender(&mut self, gender: u8) -> &mut Self {
		self.gender = gender;
		self
	}

	#[inline]
	pub fn add(&mut self, parent_index: SegmentIndex, attachment_index_offset: isize, shape: &Shape,
	           flags: segment::Flags)
	           -> &mut Self {
		self.addw(parent_index,
		          attachment_index_offset,
		          shape,
		          Winding::CW,
		          flags | segment::MIDDLE)
	}
	#[inline]
	pub fn addl(&mut self, parent_index: SegmentIndex, attachment_index_offset: isize, shape: &Shape,
	            flags: segment::Flags)
	            -> &mut Self {
		self.addw(parent_index,
		          attachment_index_offset,
		          shape,
		          Winding::CCW,
		          flags | segment::LEFT)
	}
	#[inline]
	pub fn addr(&mut self, parent_index: SegmentIndex, attachment_index_offset: isize, shape: &Shape,
	            flags: segment::Flags)
	            -> &mut Self {
		self.addw(parent_index,
		          attachment_index_offset,
		          shape,
		          Winding::CW,
		          flags | segment::RIGHT)
	}

	pub fn addw(&mut self, parent_index: SegmentIndex, attachment_index_offset: isize, shape: &Shape,
	            winding: Winding, flags: segment::Flags)
	            -> &mut Self {
		let parent = self.segments[parent_index as usize].clone();//urgh!;
		let parent_pos = parent.transform.position;
		let parent_angle = parent.transform.angle;
		let parent_length = parent.mesh.shape.length() as isize;
		let attachment_index = ((attachment_index_offset + parent_length) % parent_length) as usize;
		let spoke = parent.mesh.vertices[attachment_index];
		let p0 = cgmath::Matrix2::from_angle(cgmath::rad(parent_angle)) * spoke;
		let angle = f32::atan2(p0.y, p0.x);
		let r0 = spoke.length() * parent.mesh.shape.radius();
		let r1 = shape.radius();
		let segment = self.new_segment(shape,
		                               winding,
		                               &Transform::new(parent_pos + (p0.normalize_to(r0 + r1)),
		                                               consts::PI / 2. + angle),
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

	fn new_segment(&mut self, shape: &Shape, winding: Winding, transform: &Transform, motion: Option<&Motion>,
	               attachment: Option<segment::Attachment>, flags: segment::Flags)
	               -> segment::Segment {
		segment::Segment {
			index: self.segments.len() as SegmentIndex,
			transform: transform.clone(),
			motion: motion.map(|m| m.clone()),
			mesh: Mesh::from_shape(shape.clone(), winding),
			material: self.material.clone(),
			livery: self.livery.clone(),
			state: self.state.clone(),
			attached_to: attachment,
			flags: flags,
		}
	}

	pub fn build(&self) -> Agent {
		Agent::new(self.id,
		           self.gender,
		           &self.brain,
		           &self.dna,
		           self.segments.clone().into_boxed_slice())
	}
}
