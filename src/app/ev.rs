pub enum Event {
	CamUp,
	CamDown,
	CamLeft,
	CamRight,
	
	NextLight,
	PrevLight,
	
	NextBackground,
	PrevBackground,
	
	Reload,
	
	AppQuit,
	
	MoveLight(f32, f32),
	NewMinion(f32, f32),
	
	NoEvent,
}

