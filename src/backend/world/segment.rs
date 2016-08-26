use backend::obj;
use backend::obj::*;
use core::math;
use core::math::Smooth;
use core::geometry::*;

#[derive(Clone)]
pub enum Intent {
	Idle,
	Move(Position),
	RunAway(Position),
}

#[derive(Clone)]
pub struct State {
	age_seconds: f32,
	age_frames: usize,
	charge: f32,
	target_charge: f32,
	recharge: f32,
	smooth: math::Exponential<f32, f32>,
	pub intent: Intent,
	pub collision_detected: bool,
}

impl Default for State {
	fn default() -> Self {
		State {
			age_seconds: 0.,
			age_frames: 0,
			charge: 1.,
			target_charge: 0.,
			recharge: 1.,
			smooth: math::Exponential::new(1., 1., 2.),
			intent: Intent::Idle,
			collision_detected: false,
		}
	}
}

impl State {
	pub fn get_charge(&self) -> f32 {
		self.charge
	}

	pub fn set_charge(&mut self, charge: f32) {
		self.charge = charge;
		self.smooth.reset(self.charge);
	}

	pub fn set_target_charge(&mut self, target_charge: f32) {
		self.target_charge = target_charge;
	}

	pub fn update(&mut self, dt: f32) {
		self.age_seconds += dt;
		self.age_frames += 1;
		self.charge = self.smooth.dt(dt).smooth(self.target_charge);
		if (self.charge - self.target_charge).abs() < 0.001 {
			let reset = self.recharge;
			self.set_charge(reset);
		}
	}

	pub fn with_charge(initial: f32, target: f32, recharge: f32) -> Self {
		State {
			charge: initial,
			target_charge: target,
			recharge: recharge,
			smooth: math::Exponential::new(initial, 1., 2.),
			..Self::default()
		}
	}
}

#[derive(Copy, Clone)]
pub struct Attachment {
	pub index: SegmentIndex,
	pub attachment_point: AttachmentIndex,
}

bitflags! {
		pub flags SegmentFlags: u32 {
			const SENSOR       = 0x1,
			const JOINT        = 0x4,
			const HEAD		   = 0x10,
			const LEG          = 0x20,
			const ARM          = 0x40,
			const TORSO        = 0x100,
			const BELLY        = 0x200,
			const TAIL         = 0x400,
			const LEFT         = 0x1000,
			const RIGHT        = 0x2000,
			const MIDDLE       = 0x4000,
			const THRUSTER     = 0x10000,
			const RUDDER       = 0x20000,
			const BRAKE        = 0x40000,
			const ACTUATOR     = THRUSTER.bits | RUDDER.bits | BRAKE.bits,
		}
	}

#[derive(Clone)]
pub struct Segment {
	pub transform: Transform,
	pub index: SegmentIndex,
	pub mesh: Mesh,
	pub material: Material,
	pub livery: Livery,
	pub attached_to: Option<Attachment>,
	pub state: State,
	pub flags: SegmentFlags,
}

impl Segment {
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

impl obj::Drawable for Segment {
	fn color(&self) -> Rgba {
		let rgba = self.livery.albedo;
		let c = 5. * ((self.state.charge * 0.99) + 0.01);
		[rgba[0] * c, rgba[1] * c, rgba[2] * c, rgba[3] * self.material.density]
	}
}
