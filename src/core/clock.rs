use std::time::*;

#[derive(Clone, Debug)]
pub struct StopWatch {
	t0: SystemTime,
}

impl StopWatch {
	fn new() -> Self {
		let now = SystemTime::now();
		StopWatch { t0: now }
	}

	fn seconds(&self) -> f32 {
		match self.t0.elapsed() {
			Ok(dt) => (dt.as_secs() as f32) + (dt.subsec_nanos() as f32) * 1e-9,
			Err(_) => 0.0,
		}
	}

	fn restart(&mut self) -> f32 {
		let elapsed = self.seconds();
		self.t0 = SystemTime::now();
		elapsed
	}
}
