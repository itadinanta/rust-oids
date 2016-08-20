
pub struct Cycle<T: Copy> {
	items: Box<[T]>,
	index: usize,
}

impl<T> Cycle<T>
    where T: Copy
{
	pub fn new(items: &[T]) -> Cycle<T> {
		Cycle {
			items: items.to_vec().into_boxed_slice(),
			index: 0,
		}
	}

	pub fn get(&self) -> T {
		self.items[self.index]
	}

	pub fn next(&mut self) -> T {
		self.index = (self.index + 1) % self.items.len();
		self.items[self.index]
	}

	pub fn prev(&mut self) -> T {
		self.index = (self.index + self.items.len() - 1) % self.items.len();
		self.items[self.index]
	}
}
