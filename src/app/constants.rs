use frontend::input::AxisValue;
use core::clock::SecondsValue;

pub const FRAME_SMOOTH_COUNT: usize = 120;
pub const DEAD_ZONE: AxisValue = 0.3f32;
pub const DEBUG_DRAW_BRAKE_SCALE: f32 = 0.05;
pub const DEBUG_DRAW_MOVE_SCALE: f32 = 0.05;
pub const MIN_FRAME_LENGTH: SecondsValue = (1.0 / 1000.0) as SecondsValue;
pub const MAX_FRAME_LENGTH: SecondsValue = (1.0 / 15.0) as SecondsValue;
pub const THRUST_POWER: f32 = 1500.;
pub const POWER_BOOST: f32 = 100.;
