use backend::obj::*;
use std::f32::consts;
use core::color;
use core::color::ToRgb;
use core::geometry::*;
use core::geometry::Transform;
use core::clock;
use core::clock::SimulationTimer;
use core::clock::SharedTimer;
use backend::world::segment;
use backend::world::segment::*;
use backend::world::agent;
use backend::world::agent::N_WEIGHTS;
use backend::world::agent::Agent;
use backend::world::agent::Brain;
use backend::world::agent::TypedBrain;
use backend::world::gen::*;
use cgmath;
use cgmath::InnerSpace;

pub trait Phenotype {
	fn develop(gen: &mut Genome, id: Id, transform: &Transform, motion: Option<&Motion>, charge: f32, clock: SharedTimer<SimulationTimer>) -> agent::Agent;
}

pub struct Resource {}

pub struct Minion {}

pub struct Player {}

pub struct Spore {}

impl Phenotype for Resource {
	fn develop(gen: &mut Genome, id: Id, transform: &Transform, motion: Option<&Motion>, charge: f32, clock: SharedTimer<SimulationTimer>) -> agent::Agent {
		gen.next_integer::<u8>(0, 3);
		let albedo = color::YPbPr::new(0.5, gen.next_float(-0.5, 0.5), gen.next_float(-0.5, 0.5));
		let body = gen.eq_triangle();
		let mut builder = AgentBuilder::new(
			id,
			Material {
				density: 1.0,
				..Default::default()
			},
			Livery {
				albedo: albedo.to_rgba(),
				..Default::default()
			},
			gen.dna(),
			segment::State::with_charge(charge, 0., charge),
			clock,
		);
		builder.start(transform, motion, &body).build()
	}
}


impl Phenotype for Player {
	fn develop(gen: &mut Genome, id: Id, transform: &Transform, motion: Option<&Motion>, charge: f32, clock: SharedTimer<SimulationTimer>) -> agent::Agent {
		let albedo = color::YPbPr::new(0.5, 0., 0.);
		let body = Shape::new_star(10, 3.0, 0.9, 1./0.9);
		let mut builder = AgentBuilder::new(
			id,
			Material {
				density: 1.0,
				..Default::default()
			},
			Livery {
				albedo: albedo.to_rgba(),
				..Default::default()
			},
			gen.dna(),
			segment::State::with_charge(charge, charge, charge),
			clock,
		);
		builder.start(transform, motion, &body).build()
	}
}


impl Phenotype for Minion {
	fn develop(gen: &mut Genome, id: Id, transform: &Transform, motion: Option<&Motion>, charge: f32, clock: SharedTimer<SimulationTimer>) -> agent::Agent {
		let gender = gen.next_integer::<u8>(0, 3);
		let tint = gen.next_float(0., 1.);
		let albedo = color::Hsl::new(tint, 0.5, 0.5);
		let mut builder = AgentBuilder::new(
			id,
			Material {
				density: 0.2,
				..Default::default()
			},
			Livery {
				albedo: albedo.to_rgba(),
				..Default::default()
			},
			gen.dna(),
			segment::State::with_charge(0., charge, charge),
			clock,
		);
		builder.gender(gender);

		// personality parameters
		let mut weights_in = [[0.; N_WEIGHTS]; N_WEIGHTS];
		let mut weights_hidden = [[0.; N_WEIGHTS]; N_WEIGHTS];
		let mut weights_out = [[0.; N_WEIGHTS]; N_WEIGHTS];
		for i in 0..N_WEIGHTS {
			for j in 0..N_WEIGHTS {
				weights_in[i][j] = gen.next_float(-4., 4.);
				weights_hidden[i][j] = gen.next_float(-4., 4.);
				weights_out[i][j] = gen.next_float(-4., 4.);
			}
		}
		builder
			.hunger(&gen.next_float(0., 0.9))
			.haste(&gen.next_float(0., 0.9))
			.prudence(&gen.next_float(0., 0.9))
			.fear(&gen.next_float(0.1, 5.))
			.rest(&gen.next_float(0.2, 1.))
			.thrust(&gen.next_float(0.2, 1.))
			.weights_in(&weights_in)
			.weights_hidden(&weights_hidden)
			.weights_out(&weights_out);
		// body plan and shape
		let torso_shape = gen.any_poly();
		let torso = builder.start(transform, motion, &torso_shape).index();
		let head_shape = gen.iso_triangle();
		let tail_shape = gen.vbar();
		let i = ::std::cmp::max((torso_shape.length() as isize / 5), 1);
		builder
			.addr(
				torso,
				i,
				&gen.star(),
				Flags::ARM | Flags::JOINT | Flags::ACTUATOR | Flags::RUDDER,
			)
			.addl(
				torso,
				-i,
				&gen.star(),
				Flags::ARM | Flags::JOINT | Flags::ACTUATOR | Flags::RUDDER,
			);

		let head = builder
			.add(
				torso,
				0,
				&head_shape,
				Flags::HEAD | Flags::MOUTH | Flags::SENSOR | Flags::TRACKER,
			)
			.index();
		builder
			.addr(
				head,
				1,
				&gen.triangle(),
				Flags::HEAD | Flags::ACTUATOR | Flags::RUDDER,
			)
			.addl(
				head,
				-1,
				&gen.triangle(),
				Flags::HEAD | Flags::ACTUATOR | Flags::RUDDER,
			);

		let mut belly = torso;
		let mut belly_mid = torso_shape.mid();
		while gen.next_integer(0, 3) == 0 {
			let belly_shape = gen.any_poly();

			belly = builder
				.add(
					belly,
					belly_mid,
					&belly_shape,
					Flags::STORAGE | Flags::JOINT,
				)
				.index();
			belly_mid = belly_shape.mid();
			if belly_shape.length() > 6 {
				if gen.next_integer(0, 1) == 0 {
					builder.addr(
						belly,
						2,
						&gen.star(),
						Flags::ARM | Flags::ACTUATOR | Flags::RUDDER,
					);
				}
				if gen.next_integer(0, 1) == 0 {
					builder.addl(
						belly,
						-2,
						&gen.star(),
						Flags::ARM | Flags::ACTUATOR | Flags::RUDDER,
					);
				}
			}
			if belly > 20 {
				break;
			}
		}
		let leg_shape = gen.star();
		builder
			.addr(
				belly,
				belly_mid - 1,
				&leg_shape,
				Flags::LEG | Flags::ACTUATOR | Flags::THRUSTER,
			)
			.addl(
				belly,
				1 - belly_mid,
				&leg_shape,
				Flags::LEG | Flags::ACTUATOR | Flags::THRUSTER,
			)
			.add(
				belly,
				belly_mid,
				&tail_shape,
				Flags::TAIL | Flags::ACTUATOR | Flags::BRAKE,
			)
			.build()
	}
}

impl Phenotype for Spore {
	fn develop(gen: &mut Genome, id: Id, transform: &Transform, motion: Option<&Motion>, charge: f32, clock: SharedTimer<SimulationTimer>) -> agent::Agent {
		let gender = gen.next_integer::<u8>(0, 3);
		let tint = gen.next_float(0., 1.);
		let albedo = color::Hsl::new(tint, 0.5, 0.5);

		let mut builder = AgentBuilder::new(
			id,
			Material {
				density: 0.5,
				..Default::default()
			},
			Livery {
				albedo: albedo.to_rgba(),
				..Default::default()
			},
			gen.dna(),
			segment::State::with_charge(0., charge, charge),
			clock,
		);
		builder
			.gender(gender)
			.start(transform, motion, &gen.ball())
			.build()
	}
}

pub struct AgentBuilder {
	id: Id,
	material: Material,
	livery: Livery,
	gender: u8,
	brain: Brain,
	dna: Dna,
	state: segment::State,
	segments: Vec<Segment>,
	clock: SharedTimer<clock::SimulationTimer>,
}

impl AgentBuilder {
	pub fn new(id: Id, material: Material, livery: Livery, dna: &Dna, state: segment::State, clock: SharedTimer<clock::SimulationTimer>) -> Self {
		AgentBuilder {
			id,
			material,
			livery,
			state,
			gender: 0u8,
			brain: Brain::default(),
			dna: dna.clone(),
			segments: Vec::new(),
			clock,
		}
	}

	pub fn start(&mut self, transform: &Transform, motion: Option<&Motion>, shape: &Shape) -> &mut Self {
		let segment = self.new_segment(
			shape,
			Winding::CW,
			transform,
			motion,
			None,
			segment::Flags::CORE | segment::Flags::STORAGE | segment::Flags::MIDDLE,
		);
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
	pub fn add(&mut self, parent_index: SegmentIndex, attachment_index_offset: isize, shape: &Shape, flags: segment::Flags)
			   -> &mut Self {
		self.addw(
			parent_index,
			attachment_index_offset,
			shape,
			Winding::CW,
			flags | segment::Flags::MIDDLE,
		)
	}
	#[inline]
	pub fn addl(&mut self, parent_index: SegmentIndex, attachment_index_offset: isize, shape: &Shape, flags: segment::Flags)
				-> &mut Self {
		self.addw(
			parent_index,
			attachment_index_offset,
			shape,
			Winding::CCW,
			flags | segment::Flags::LEFT,
		)
	}
	#[inline]
	pub fn addr(&mut self, parent_index: SegmentIndex, attachment_index_offset: isize, shape: &Shape, flags: segment::Flags)
				-> &mut Self {
		self.addw(
			parent_index,
			attachment_index_offset,
			shape,
			Winding::CW,
			flags | segment::Flags::RIGHT,
		)
	}

	pub fn addw(
		&mut self, parent_index: SegmentIndex, attachment_index_offset: isize, shape: &Shape, winding: Winding,
		flags: segment::Flags,
	) -> &mut Self {
		let parent = self.segments[parent_index as usize].clone(); //urgh!;
		let parent_pos = parent.transform.position;
		let parent_angle = parent.transform.angle;
		let parent_length = parent.mesh.shape.length() as isize;
		let attachment_index = ((attachment_index_offset + parent_length) % parent_length) as usize;
		let spoke = parent.mesh.vertices[attachment_index];
		let p0 = cgmath::Matrix2::from_angle(cgmath::Rad(parent_angle)) * spoke;
		let angle = f32::atan2(p0.y, p0.x);
		let r0 = spoke.magnitude() * parent.mesh.shape.radius();
		let r1 = shape.radius();
		let segment = self.new_segment(
			shape,
			winding,
			&Transform::new(
				parent_pos + (p0.normalize_to(r0 + r1)),
				consts::PI / 2. + angle,
			),
			None,
			parent.new_attachment(attachment_index as AttachmentIndex),
			flags,
		);
		self.segments.push(segment);
		self
	}

	pub fn index(&self) -> SegmentIndex {
		match self.segments.len() {
			0 => 0,
			n => (n - 1) as SegmentIndex,
		}
	}

	pub fn hunger(&mut self, value: &<Brain as TypedBrain>::Parameter) -> &mut Self {
		self.brain.hunger = value.clone();
		self
	}

	pub fn haste(&mut self, value: &<Brain as TypedBrain>::Parameter) -> &mut Self {
		self.brain.haste = value.clone();
		self
	}

	pub fn prudence(&mut self, value: &<Brain as TypedBrain>::Parameter) -> &mut Self {
		self.brain.prudence = value.clone();
		self
	}

	pub fn fear(&mut self, value: &<Brain as TypedBrain>::Parameter) -> &mut Self {
		self.brain.fear = value.clone();
		self
	}

	pub fn rest(&mut self, value: &<Brain as TypedBrain>::Parameter) -> &mut Self {
		self.brain.rest = value.clone();
		self
	}

	pub fn thrust(&mut self, value: &<Brain as TypedBrain>::Parameter) -> &mut Self {
		self.brain.thrust = value.clone();
		self
	}

	pub fn weights_in(&mut self, weights_in: &<Brain as TypedBrain>::WeightMatrix) -> &mut Self {
		self.brain.weights_in = weights_in.clone();
		self
	}

	pub fn weights_hidden(&mut self, weights_hidden: &<Brain as TypedBrain>::WeightMatrix) -> &mut Self {
		self.brain.weights_hidden = weights_hidden.clone();
		self
	}

	pub fn weights_out(&mut self, weights_out: &<Brain as TypedBrain>::WeightMatrix) -> &mut Self {
		self.brain.weights_out = weights_out.clone();
		self
	}

	fn new_segment(
		&mut self, shape: &Shape, winding: Winding, transform: &Transform, motion: Option<&Motion>,
		attachment: Option<segment::Attachment>, flags: segment::Flags,
	) -> segment::Segment {
		segment::Segment {
			index: self.segments.len() as SegmentIndex,
			transform: transform.clone(),
			motion: motion.map(|m| m.clone()),
			mesh: Mesh::from_shape(shape.clone(), winding),
			material: self.material.clone(),
			livery: self.livery.clone(),
			state: self.state.clone(),
			attached_to: attachment,
			flags,
		}
	}

	pub fn build(&self) -> Agent {
		trace!("Agent {:?} has brain {:?}", self.id, self.brain);
		Agent::new(
			self.id,
			self.gender,
			&self.brain,
			&self.dna,
			self.segments.clone().into_boxed_slice(),
			self.clock.clone(),
		)
	}
}
