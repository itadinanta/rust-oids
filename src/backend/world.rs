use backend::obj;
use backend::obj::*;
use rand;
use rand::Rng;
use std::collections::HashMap;
use std::slice;
use std::f32::consts;
use cgmath;
use cgmath::Rotation;
use cgmath::EuclideanVector;
use num;

#[derive(Clone)]
pub struct State {
	age_seconds: f32,
	age_frames: usize,
	charge: f32,
	target_charge: f32,
	tau: f32,
}

impl Default for State {
	fn default() -> Self {
		State {
			age_seconds: 0.,
			age_frames: 0,
			charge: 1.,
			target_charge: 0.,
			tau: 2.0,
		}
	}
}

impl State {
	pub fn update(&mut self, dt: f32) {
		self.age_seconds += dt;
		self.age_frames += 1;
		let alpha = 1. - f32::exp(-dt / self.tau);
		self.charge = self.target_charge * alpha + self.charge * (1. - alpha);
	}

	pub fn with_charge(initial: f32, target: f32) -> Self {
		State {
			charge: initial,
			target_charge: target,
			..Self::default()
		}
	}

	pub fn charge(&self) -> f32 {
		self.charge
	}
}

#[derive(Copy, Clone)]
pub struct Attachment {
	pub index: LimbIndex,
	pub attachment_point: AttachmentIndex,
}

#[derive(Clone)]
pub struct Limb {
	transform: Transform,
	index: LimbIndex,
	mesh: Mesh,
	material: Material,
	pub attached_to: Option<Attachment>,
	pub state: State,
}

impl Limb {
	pub fn new_attachment(&self, attachment_point: AttachmentIndex) -> Option<Attachment> {
		let max = self.mesh.vertices.len() as AttachmentIndex;
		Some(Attachment {
			index: self.index,
			attachment_point: if attachment_point < max {
				attachment_point
			} else {
				max - 1
			},
		})
	}
}

pub struct Creature {
	id: Id,
	limbs: Vec<Limb>,
}

impl GameObject for Creature {
	fn id(&self) -> Id {
		self.id
	}
}

impl Transformable for Creature {
	fn transform(&self) -> Transform {
		self.limbs.first().unwrap().transform()
	}
	fn transform_to(&mut self, t: Transform) {
		self.limbs.first_mut().unwrap().transform_to(t);
	}
}

impl Transformable for Limb {
	fn transform(&self) -> Transform {
		self.transform
	}
	fn transform_to(&mut self, t: Transform) {
		self.transform = t;
	}
}

impl obj::Geometry for Limb {
	fn mesh(&self) -> &Mesh {
		&self.mesh
	}
}

impl obj::Solid for Limb {
	fn material(&self) -> Material {
		self.material
	}
}

impl obj::Drawable for Limb {
	fn color(&self) -> Rgba {
		// let lightness = 1. - self.material.density * 0.5;
		// [0., 10. * lightness, 0., 1.]
		[9. * self.state.charge + 0.1, 4. * self.state.charge, 0., 1.]
	}
}

impl Creature {
	pub fn id(&self) -> Id {
		self.id
	}

	pub fn limbs(&self) -> slice::Iter<Limb> {
		self.limbs.iter()
	}

	pub fn limbs_mut(&mut self) -> slice::IterMut<Limb> {
		self.limbs.iter_mut()
	}

	pub fn limb_mut(&mut self, index: LimbIndex) -> Option<&mut Limb> {
		self.limbs.get_mut(index as usize)
	}
}

struct CreatureBuilder {
	rng: rand::ThreadRng,
	material: Material,
	state: State,
	limbs: Vec<Limb>,
}

use std::ops::{Add, Mul, Sub};

impl CreatureBuilder {
	fn new(material: Material, state: State) -> Self {
		CreatureBuilder {
			rng: rand::thread_rng(),
			material: material,
			state: state,
			limbs: Vec::new(),
		}
	}

	fn frand<T>(&mut self, min: T, max: T) -> T
		where T: rand::Rand + num::Float {
		self.rng.gen::<T>() * (max - min) + min
	}

	fn irand<T>(&mut self, min: T, max: T) -> T
		where T: rand::Rand + num::Integer + Copy {
		self.rng.gen::<T>() % (max - min + T::one()) + min
	}

	fn random_ball(&mut self) -> Shape {
		let radius: f32 = self.frand(1.0, 2.0);
		Shape::new_ball(radius)
	}

	fn random_box(&mut self) -> Shape {
		let radius: f32 = self.frand(1.0, 2.0);
		let ratio: f32 = self.frand(1.0, 2.0);
		Shape::new_box(radius, ratio)
	}

	fn random_vbar(&mut self) -> Shape {
		let radius: f32 = self.frand(1.0, 2.0);
		let ratio: f32 = self.frand(0.1, 0.2);
		Shape::new_box(radius, ratio)
	}

	fn random_triangle(&mut self) -> Shape {
		let radius = self.frand(0.5, 1.0);
		let alpha1 = self.frand(consts::PI * 0.5, consts::PI * 0.9);
		let alpha2 = consts::PI * 1.5 - self.frand(0., consts::PI);
		Shape::new_triangle(radius, alpha1, alpha2)
	}

	fn random_iso_triangle(&mut self) -> Shape {
		let radius = self.frand(0.5, 1.0);
		let alpha1 = self.frand(consts::PI * 0.5, consts::PI * 0.9);
		let alpha2 = consts::PI * 2. - alpha1;
		Shape::new_triangle(radius, alpha1, alpha2)
	}

	fn random_eq_triangle(&mut self) -> Shape {
		let radius = self.frand(0.5, 1.0);
		let alpha1 = consts::PI * 2. / 3.;
		let alpha2 = consts::PI * 2. - alpha1;
		Shape::new_triangle(radius, alpha1, alpha2)
	}

	fn random_star(&mut self) -> Shape {
		let radius: f32 = self.frand(1.0, 2.0);
		let n = self.irand(3, 8);
		let ratio1 = self.frand(0.5, 1.0);
		let ratio2 = self.frand(0.7, 0.9) * (1. / ratio1);
		Shape::new_star(n, radius, ratio1, ratio2)
	}

	fn random_poly(&mut self, upside_down: bool) -> Shape {
		let n = self.irand(3, 8);
		self.random_npoly(n, upside_down)
	}

	fn random_npoly(&mut self, n: AttachmentIndex, upside_down: bool) -> Shape {
		let radius: f32 = self.frand(1.0, 2.0);
		let ratio1 = f32::cos(consts::PI / n as f32);
		let ratio2 = 1. / ratio1;
		if upside_down {
			Shape::new_star(n, radius * ratio1, ratio2, ratio1)
		} else {
			Shape::new_star(n, radius, ratio1, ratio2)
		}
	}

	pub fn start(&mut self, position: obj::Position, angle: f32, shape: &Shape, winding: Winding) -> &mut Self {
		let limb = self.new_limb(shape, winding, position, angle, None);
		self.limbs.clear();
		self.limbs.push(limb);
		self
	}

	pub fn add(&mut self,
	           parent_index: LimbIndex,
	           attachment_index_offset: isize,
	           shape: &Shape,
	           winding: Winding)
	           -> &mut Self {
		let parent = self.limbs[parent_index as usize].clone();//urgh!;
		let parent_pos = parent.transform.position;
		let parent_angle = parent.transform.angle;
		let parent_length = parent.mesh.shape.length() as isize;
		let attachment_index = ((attachment_index_offset + parent_length) % parent_length) as usize;
		let p0 = cgmath::Matrix2::from_angle(cgmath::rad(parent_angle)) * parent.mesh.vertices[attachment_index];
		let angle = f32::atan2(p0.y, p0.x);
		let r0 = p0.length() * parent.mesh.shape.radius();
		let r1 = shape.radius();
		let limb = self.new_limb(shape,
		                         winding,
		                         parent_pos + (p0 * (r0 + r1)),
		                         consts::PI / 2. + angle,
		                         parent.new_attachment(attachment_index as AttachmentIndex));
		self.limbs.push(limb);
		self
	}

	pub fn index(&self) -> LimbIndex {
		match self.limbs.len() {
			0 => 0,
			n => (n - 1) as LimbIndex,
		}
	}

	fn new_limb(&mut self,
	            shape: &Shape,
	            winding: Winding,
	            position: obj::Position,
	            angle: f32,
	            attachment: Option<Attachment>)
	            -> Limb {
		Limb {
			index: self.limbs.len() as LimbIndex,
			transform: obj::Transform::new(position, angle),
			mesh: Mesh::from_shape(shape.clone(), winding),
			material: self.material.clone(),
			state: self.state.clone(),
			attached_to: attachment,
		}
	}

	pub fn build(&self, id: obj::Id) -> Creature {
		Creature {
			id: id,
			limbs: self.limbs.clone(),
		}
	}
}

pub struct Flock {
	last_id: Id,
	creatures: HashMap<Id, Creature>,
}

impl Flock {
	pub fn new() -> Flock {
		Flock {
			last_id: 0,
			creatures: HashMap::new(),
		}
	}

	pub fn get(&self, id: Id) -> Option<&Creature> {
		self.creatures.get(&id)
	}

	pub fn get_mut(&mut self, id: Id) -> Option<&mut Creature> {
		self.creatures.get_mut(&id)
	}

	pub fn next_id(&mut self) -> Id {
		self.last_id = self.last_id + 1;
		self.last_id
	}

	pub fn new_creature(&mut self, initial_pos: Position, charge: f32) -> Id {
		let mut rng = rand::thread_rng();

		let mut builder =
			CreatureBuilder::new(Material { density: (rng.gen::<f32>() * 1.0) + 1.0, ..Default::default() },
			                     State::with_charge(charge, 0.));

		let id = self.next_id();
		let arm_shape = builder.random_star();
		let leg_shape = builder.random_star();
		let torso_shape = builder.random_npoly(5, true);
		let head_shape = builder.random_iso_triangle();
		let tail_shape = builder.random_vbar();
		let initial_angle = consts::PI / 2. + f32::atan2(initial_pos.y, initial_pos.x);

		let torso = builder.start(initial_pos, initial_angle, &torso_shape, Winding::CW)
			.index();

		builder.add(torso, 2, &arm_shape, Winding::CW)
			.add(torso, -2, &arm_shape, Winding::CCW)
			.add(torso, 4, &leg_shape, Winding::CW)
			.add(torso, -4, &leg_shape, Winding::CCW);

		let head = builder.add(torso, 0, &head_shape, Winding::CW).index();
		let tail = builder.add(torso, torso_shape.mid() as isize, &tail_shape, Winding::CW)
			.index();
		builder.add(head, 1, &head_shape, Winding::CW)
			.add(head, 2, &head_shape, Winding::CCW);

		let creature = builder.build(id);
		self.creatures.insert(id, creature);
		id
	}

	pub fn kill(&mut self, id: &Id) {
		self.creatures.remove(id);
	}

	pub fn creatures(&self) -> &HashMap<Id, Creature> {
		&self.creatures
	}
}

#[repr(packed)]
#[derive(Eq, Hash, PartialEq, Clone, Copy)]
pub struct CreatureRefs {
	pub creature_id: obj::Id,
	pub limb_index: obj::LimbIndex,
	pub bone_index: obj::BoneIndex,
}

impl Default for CreatureRefs {
	fn default() -> CreatureRefs {
		CreatureRefs {
			creature_id: 0xdeadbeef,
			limb_index: 0xff,
			bone_index: 0xff,
		}
	}
}

impl CreatureRefs {
	pub fn with_id(id: obj::Id) -> CreatureRefs {
		CreatureRefs { creature_id: id, ..Default::default() }
	}

	pub fn with_limb(id: obj::Id, limb_index: obj::LimbIndex) -> CreatureRefs {
		CreatureRefs {
			creature_id: id,
			limb_index: limb_index,
			..Default::default()
		}
	}

	pub fn with_bone(id: obj::Id, limb_index: obj::LimbIndex, bone_index: obj::BoneIndex) -> CreatureRefs {
		CreatureRefs {
			creature_id: id,
			limb_index: limb_index,
			bone_index: bone_index,
		}
	}
}

pub struct World {
	pub friends: Flock,
}

impl World {
	pub fn new() -> Self {
		World { friends: Flock::new() }
	}

	pub fn new_ball(&mut self, pos: obj::Position) -> obj::Id {
		self.friends.new_creature(pos, 0.3)
	}

	pub fn new_star(&mut self, pos: obj::Position) -> obj::Id {
		self.friends.new_creature(pos, 0.3)
	}

	pub fn friend(&self, id: obj::Id) -> Option<&Creature> {
		self.friends.get(id)
	}

	pub fn friend_mut(&mut self, id: obj::Id) -> Option<&mut Creature> {
		self.friends.get_mut(id)
	}
}
