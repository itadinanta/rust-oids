use core::geometry::*;
use core::clock::*;

#[derive(Clone, Copy, Debug)]
pub enum VectorDirection {
	None,
	Orientation(Position),
	LookAt(Position),
	Turn(Angle),
	FromVelocity,
}

#[derive(Clone, Copy, Debug)]
pub enum Event {
	CamUp(f32),
	CamDown(f32),
	CamLeft(f32),
	CamRight(f32),

	ZoomIn,
	ZoomOut,
	ZoomReset,

	VectorThrust(Option<Position>, VectorDirection),
	PrimaryTrigger(f32, SecondsValue),
	PrimaryFire(f32, SecondsValue),

	CamReset,

	NextLight,
	PrevLight,

	NextBackground,
	PrevBackground,

	NextSpeedFactor,
	PrevSpeedFactor,

	Reload,
	SaveGenePoolToFile,
	SaveWorldToFile,
	RestartFromCheckpoint,
	ToggleDebug,

	TogglePause,
	ToggleGui,

	AppQuit,

	NewMinion(Position),
	RandomizeMinion(Position),

	PickMinion(Position),
	SelectMinion(usize),
	DeselectAll,

	BeginDrag(Position, Position),
	Drag(Position, Position),
	EndDrag(Position, Position, Velocity),
}