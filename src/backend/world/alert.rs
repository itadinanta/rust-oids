use core::clock::Seconds;

#[allow(unused)]
#[derive(Copy, Clone, Debug)]
pub enum Alert {
	BeginSimulation,
	NewMinion,
	NewSpore,
	NewResource,
	NewBullet(usize),
	DieMinion,
	DieResource,
	Fertilised,
}
