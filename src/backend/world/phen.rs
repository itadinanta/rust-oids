use backend::obj::*;
use std::f32::consts;
use core::color;
use core::color::ToRgb;
use core::geometry::*;
use backend::world::segment;
use backend::world::segment::*;
use backend::world::agent;
use backend::world::gen::*;

pub trait Phenotype {
	fn develop(gen: &mut Genome,
	           id: Id,
	           initial_pos: Position,
	           initial_vel: Option<Motion>,
	           charge: f32)
	           -> agent::Agent;
}

pub struct Resource {}

impl Phenotype for Resource {
	fn develop(gen: &mut Genome,
	           id: Id,
	           initial_pos: Position,
	           initial_vel: Option<Motion>,
	           charge: f32)
	           -> agent::Agent {

		let albedo = color::YPbPr::new(0.5, gen.next_float(-0.5, 0.5), gen.next_float(-0.5, 0.5));
		let ball = gen.ball();
		let mut builder = agent::AgentBuilder::new(id,
		                                           Material { density: 1.0, ..Default::default() },
		                                           Livery { albedo: albedo.to_rgba(), ..Default::default() },
		                                           gen.dna(),
		                                           segment::State::with_charge(charge, 0., charge));
		builder.start(Transform::with_position(initial_pos), initial_vel, &ball).build()
	}
}

pub struct Minion {}

impl Phenotype for Minion {
	fn develop(gen: &mut Genome,
	           id: Id,
	           initial_pos: Position,
	           initial_vel: Option<Motion>,
	           charge: f32)
	           -> agent::Agent {
		let albedo = color::Hsl::new(gen.next_float(0., 1.), 0.5, 0.5);
		let mut builder = agent::AgentBuilder::new(id,
		                                           Material { density: 0.2, ..Default::default() },
		                                           Livery { albedo: albedo.to_rgba(), ..Default::default() },
		                                           gen.dna(),
		                                           segment::State::with_charge(0., charge, charge));
		let arm_shape = gen.star();
		let leg_shape = gen.star();
		let torso_shape = gen.npoly(5, true);
		let head_shape = gen.iso_triangle();
		let tail_shape = gen.vbar();
		let initial_angle = consts::PI / 2. + f32::atan2(initial_pos.y, initial_pos.x);

		let torso = builder.start(Transform::new(initial_pos, initial_angle),
			       initial_vel,
			       &torso_shape)
			.index();
		builder.addr(torso, 2, &arm_shape, ARM | JOINT | ACTUATOR | RUDDER)
			.addl(torso, -2, &arm_shape, ARM | JOINT | ACTUATOR | RUDDER);

		let head = builder.add(torso, 0, &head_shape, HEAD | SENSOR).index();
		builder.addr(head, 1, &head_shape, HEAD | ACTUATOR | RUDDER)
			.addl(head, 2, &head_shape, HEAD | ACTUATOR | RUDDER);

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
