use backend::obj;
use backend::obj::*;
use backend::world::agent;
use core::clock::Seconds;
use core::geometry::Transform;
use core::geometry::*;
use core::math;
use core::math::ExponentialFilter;
use num::Zero;

#[derive(Clone)]
pub enum PilotRotation {
	None,
	Orientation(Position),
	LookAt(Position),
	Turn(Angle),
	FromVelocity,
}

#[derive(Clone)]
pub enum Intent {
	Idle,
	Move(Position),
	Brake(Position),
	RunAway(Position),
	PilotTo(Option<Position>, PilotRotation),
}

#[derive(Clone)]
pub struct State {
	age_seconds: Seconds,
	age_frames: usize,
	maturity: f32,
	charge: ExponentialFilter<f32>,
	pub intent: Intent,
	pub last_touched: Option<agent::Key>,
}

impl Default for State {
	fn default() -> Self {
		State {
			age_seconds: Seconds::zero(),
			age_frames: 0,
			maturity: 1.0,
			charge: math::exponential_filter(0., 1., 2.),
			intent: Intent::Idle,
			last_touched: None,
		}
	}
}

impl State {
	pub fn age_seconds(&self) -> Seconds { self.age_seconds }
	pub fn age_frames(&self) -> usize { self.age_frames }

	pub fn charge(&self) -> f32 { self.charge.get() }
	pub fn reset_charge(&mut self, current_charge: f32, target_charge: f32) {
		self.charge.reset_to(target_charge, current_charge);
	}
	pub fn set_output_charge(&mut self, charge: f32) { self.charge.force_to(charge); }
	pub fn target_charge(&self) -> f32 { self.charge.last_input() }
	pub fn set_target_charge(&mut self, target_charge: f32) { self.charge.input(target_charge); }

	//pub fn recharge(&self) -> f32 { self.recharge }

	pub fn maturity(&self) -> f32 { self.maturity }

	pub fn set_maturity(&mut self, maturity: f32) { self.maturity = maturity }

	pub fn restore(&mut self, current_charge: f32, target_charge: f32) {
		self.charge.reset_to(target_charge, current_charge);
	}

	pub fn update(&mut self, dt: Seconds) {
		self.age_seconds += dt;
		self.age_frames += 1;
		self.charge.update(dt.into());
	}

	pub fn with_charge(current_charge: f32, target_charge: f32, charge_decay_time: Seconds) -> Self {
		State {
			charge: math::exponential_filter(target_charge, current_charge, charge_decay_time.get() as f32),
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
	pub struct Flags: u32 {
		const SENSOR       = 0x00001u32;
		const ACTUATOR     = 0x00002u32;
		const JOINT        = 0x00004u32;
		const MOUTH		   = 0x00008u32;
		const HEAD		   = 0x00010u32;
		const LEG          = 0x00020u32;
		const ARM          = 0x00040u32;
		const CORE         = 0x00100u32;
		const STORAGE      = 0x00200u32;
		const TAIL         = 0x00400u32;
		const TRACKER	   = 0x00800u32;
		const LEFT         = 0x01000u32;
		const RIGHT        = 0x02000u32;
		const MIDDLE       = 0x04000u32;
		const THRUSTER     = 0x10000u32;
		const RUDDER       = 0x20000u32;
		const BRAKE        = 0x40000u32;
	}
}

#[derive(Clone)]
pub struct Segment {
	pub transform: Transform,
	pub rest_angle: Angle,
	pub motion: Motion,
	pub index: SegmentIndex,
	pub mesh: Mesh,
	pub material: Material,
	pub livery: Livery,
	pub attached_to: Option<Attachment>,
	pub state: State,
	pub flags: Flags,
}

impl Segment {
	pub fn new_attachment(&self, attachment_point: AttachmentIndex) -> Option<Attachment> {
		let max = self.mesh.vertices.len() as AttachmentIndex;
		Some(Attachment {
			index: self.index,
			attachment_point: if attachment_point < max { attachment_point } else { max - 1 },
		})
	}

	pub fn growing_radius(&self) -> f32 { self.state.maturity * self.mesh.shape.radius() }

	pub fn growing_scaled_vertex(&self, index: usize) -> Position {
		self.state.maturity * self.mesh.scaled_vertex(index)
	}
}

impl obj::Drawable for Segment {
	fn color(&self) -> Rgba {
		let rgba = self.livery.albedo;
		let c = 5. * ((self.state.charge.get() * 0.99) + 0.01);
		[rgba[0] * c, rgba[1] * c, rgba[2] * c, rgba[3] * self.material.density]
	}
}

impl Transformable for Segment {
	fn transform(&self) -> &Transform { &self.transform }

	fn transform_to(&mut self, t: Transform) { self.transform = t; }
}

impl Motionable for Segment {
	fn motion(&self) -> &Motion { &self.motion }

	fn motion_to(&mut self, m: Motion) { self.motion = m; }
}

impl obj::Geometry for Segment {
	fn mesh(&self) -> &Mesh { &self.mesh }
}

impl obj::Solid for Segment {
	fn material(&self) -> &Material { &self.material }

	fn livery(&self) -> &Livery { &self.livery }
}
