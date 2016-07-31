
pub struct Smooth<S: ::num::Num> {
	ptr: usize,
	count: usize,
	acc: S,
	last: S,
	values: Vec<S>,
}

impl<S: ::num::Num + ::num::NumCast + ::std::marker::Copy> Smooth<S> {
	pub fn new(window_size: usize) -> Smooth<S> {
		Smooth {
			ptr: 0,
			count: 0,
			last: S::zero(),
			acc: S::zero(),
			values: vec![S::zero(); window_size],
		}
	}

	pub fn smooth(&mut self, value: S) -> S {
		let len = self.values.len();
		if self.count < len {
			self.count = self.count + 1;
		} else {
			self.acc = self.acc - self.values[self.ptr];
		}
		self.acc = self.acc + value;
		self.values[self.ptr] = value;
		self.ptr = ((self.ptr + 1) % len) as usize;
		self.last = self.acc / ::num::cast(self.count).unwrap();
		self.last
	}
}
