#[derive(Clone, Debug)]
pub struct History<T: Clone> {
	values: Vec<T>,
	count: usize,
	ptr: usize,
}

pub trait Initial {
	fn initial() -> Self;
}

impl<T> History<T>
where
	T: Clone + Initial,
{
	pub fn new(n: usize) -> Self {
		History {
			values: vec![T::initial(); n],
			count: 0,
			ptr: 0,
		}
	}

	pub fn push(&mut self, value: T) {
		let len = self.values.len();
		if self.count < len {
			self.count = self.count + 1;
		}
		self.values[self.ptr] = value;
		self.ptr = (self.ptr + 1) % len;
	}
}

pub struct HistoryIntoIterator<'a, T>
where
	T: Clone + Initial + 'a,
{
	history: &'a History<T>,
	index: usize,
}

impl<'a, T> IntoIterator for &'a History<T>
where
	T: Clone + Initial,
{
	type Item = T;
	type IntoIter = HistoryIntoIterator<'a, T>;

	fn into_iter(self) -> Self::IntoIter {
		HistoryIntoIterator {
			history: self,
			index: 0,
		}
	}
}

impl<'a, T> Iterator for HistoryIntoIterator<'a, T>
where
	T: Clone + Initial,
{
	type Item = T;

	fn next(&mut self) -> Option<T> {
		if self.index >= self.history.count {
			None
		} else {
			let len = self.history.values.len();
			let item = self.history.values[(len + self.history.ptr - 1 - self.index) % len].clone();
			self.index += 1;
			Some(item)
		}
	}
}

pub struct Cycle<T: Copy> {
	items: Box<[T]>,
	index: usize,
}

impl<T> Cycle<T>
where
	T: Copy,
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
