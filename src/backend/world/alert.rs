#[allow(unused)]
#[derive(Copy, Clone, Debug)]
pub enum Alert {
	BeginSimulation,
	RestartFromCheckpoint,
	NewMinion,
	NewSpore,
	NewResource,
	NewBullet(usize),
	DieMinion,
	DieResource,
	Fertilised,
	GrowMinion,
}
