use core::geometry::Position;
pub enum Event {
	CamUp,
	CamDown,
	CamLeft,
	CamRight,

	CamReset,

	NextLight,
	PrevLight,

	NextBackground,
	PrevBackground,

	Reload,

	AppQuit,

	MoveLight(Position),
	NewMinion(Position),
	NewResource(Position),

	NoEvent,
}
