use core::geometry::*;
use core::clock::*;

#[derive(Clone, Copy, Debug)]
pub enum VectorDirection {
	None,
	Orientation(Position),
	LookAt(Position),
	Turn(Angle),
}

#[derive(Clone, Copy, Debug)]
pub enum Event {
	CamUp(f32),
	CamDown(f32),
	CamLeft(f32),
	CamRight(f32),

	VectorThrust(Option<Position>, VectorDirection),
	PrimaryFire(f32, SecondsValue),

	CamReset,

	NextLight,
	PrevLight,

	NextBackground,
	PrevBackground,

	NextSpeedFactor,
	PrevSpeedFactor,

	Reload,
	DumpToFile,
	ToggleDebug,

	TogglePause,
	ToggleGui,

	AppQuit,

	NewMinion(Position),
	RandomizeMinion(Position),

	PickMinion(Position),
	DeselectAll,

	BeginDrag(Position, Position),
	Drag(Position, Position),
	EndDrag(Position, Position, Velocity),
}