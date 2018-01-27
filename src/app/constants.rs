use frontend::input::AxisValue;
use core::clock::{SpeedFactor, SecondsValue};
use std::f32::consts;

pub const FRAME_SMOOTH_COUNT: usize = 120;
pub const DEAD_ZONE: AxisValue = 0.3f32;
pub const TURN_SPEED: f32 = consts::PI * 200.;
pub const DEBUG_DRAW_BRAKE_SCALE: f32 = 0.05;
pub const DEBUG_DRAW_MOVE_SCALE: f32 = 0.05;
pub const MIN_FRAME_LENGTH: SecondsValue = (1.0 / 1000.0) as SecondsValue;
pub const MAX_FRAME_LENGTH: SecondsValue = (1.0 / 15.0) as SecondsValue;
pub const THRUST_POWER: f32 = 5000.;
pub const POWER_BOOST: f32 = 100.;
pub const DRAG_COEFFICIENT: f32 = 0.000001;
pub const COMPASS_SPRING_POWER: f32 = 1000.0;
pub const JOINT_UPPER_ANGLE: f32 = consts::PI / 6.;
pub const JOINT_LOWER_ANGLE: f32 = -consts::PI / 6.;
pub const JOINT_FREQUENCY: f32 = 5.0;
pub const JOINT_DAMPING_RATIO: f32 = 0.9;
pub const LINEAR_DAMPING_DEFAULT: f32 = 0.8;
pub const LINEAR_DAMPING_PLAYER: f32 = 2.0;
pub const ANGULAR_DAMPING: f32 = 0.9;
pub const PICK_EPS: f32 = 0.001f32;
pub const DEFAULT_RESOURCE_CHARGE: f32 = 0.8;
pub const DEFAULT_SPORE_CHARGE: f32 = 0.8;
pub const DEFAULT_MINION_CHARGE: f32 = 0.3;
pub const INITIAL_SPAWN_RADIUS_RATIO: f32 = 0.25;
pub const INITIAL_SPAWN_RADIUS_SLICES: f32 = 16.;
pub const WORLD_RADIUS: f32 = 80.;
pub const EMITTER_DISTANCE: f32 = 20.;
pub const EMITTER_PERIOD: SecondsValue = 0.4;
pub const EMITTER_SPREAD_ANGLE: f32 = consts::PI / 12.;
pub const BULLET_SPEED_SCALE: f32 = 100.;
pub const BULLET_FIRE_RATE_SCALE: SecondsValue = 0.5;
pub const BULLET_FULL_CHARGE: SecondsValue = 1.0;
pub const BULLET_FIRE_RATE: SecondsValue = 45.0;
pub const DENSITY_DEFAULT: f32 = 1.0;
pub const DENSITY_RESOURCE: f32 = DENSITY_DEFAULT;
pub const DENSITY_PLAYER: f32 = 1.0;
pub const DENSITY_MINION: f32 = 0.2;
pub const DENSITY_SPORE: f32 = 0.5;
pub const RESTITUTION_DEFAULT: f32 = 0.6;
pub const RESTITUTION_PLAYER: f32 = 0.1;
pub const FRICTION_DEFAULT: f32 = 0.7;
pub const FRICTION_PLAYER: f32 = 0.6;
pub const DEFAULT_MINION_GENE_POOL: &'static [&'static str] = &[
	"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
	"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
	"GzB2lQVwM00tTAm5gwajjf4wc0a5GzB2lQVwM00tTAm5gwajjf4wc0a5",
	"GzB2lQdwM10vQEu5zwaPgDhfq2v8GzB2lQdwM10vQEu5zwaPgDhfq2v8",
];

pub const DEFAULT_RESOURCE_GENE_POOL: &'static [&'static str] = &[
	"GyA21QoQ",
	"M00sWS0M"
];

pub const DUMP_FILE_PATTERN: &'static str = "resources/%Y%m%d_%H%M%S.csv";

pub const AMBIENT_LIGHTS: &'static [[f32; 4]] = &[
	[1.0, 1.0, 1.0, 1.0],
	[3.1, 3.1, 3.1, 1.0],
	[10.0, 10.0, 10.0, 1.0],
	[31.0, 31.0, 31.0, 1.0],
	[100.0, 100.0, 100.0, 1.0],
	[0.001, 0.001, 0.001, 1.0],
	[0.01, 0.01, 0.01, 1.0],
	[0.1, 0.1, 0.1, 1.0],
	[0.31, 0.31, 0.31, 0.5],
];

pub const SPEED_FACTORS: &'static [SpeedFactor] = &[
	1.0,
	0.5,
	0.2,
	0.1,
	1.0,
	2.0,
	5.0,
	10.0,
];

pub const BACKGROUNDS: &'static [[f32; 4]] = &[
	[0.05, 0.07, 0.1, 1.0],
	[0.5, 0.5, 0.5, 0.5],
	[1.0, 1.0, 1.0, 1.0],
	[3.1, 3.1, 3.1, 1.0],
	[10.0, 10.0, 10.0, 1.0],
	[0., 0., 0., 1.0],
	[0.01, 0.01, 0.01, 1.0],
];