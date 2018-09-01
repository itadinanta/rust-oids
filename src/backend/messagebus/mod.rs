use app::Event;
use backend::world::alert::Alert;
use backend::world::particle::Emitter;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

#[derive(Clone)]
pub enum Message {
	Alert(Alert),
	Event(Event),
	NewEmitter(Emitter),
}

impl From<Emitter> for Message {
	fn from(value: Emitter) -> Self {
		Message::NewEmitter(value)
	}
}

impl From<Event> for Message {
	fn from(value: Event) -> Self {
		Message::Event(value)
	}
}

impl From<Alert> for Message {
	fn from(value: Alert) -> Self {
		Message::Alert(value)
	}
}

impl Into<Option<Emitter>> for Message {
	fn into(self) -> Option<Emitter> {
		match self {
			Message::NewEmitter(emitter) => Some(emitter),
			_ => None,
		}
	}
}

impl Into<Option<Alert>> for Message {
	fn into(self) -> Option<Alert> {
		match self {
			Message::Alert(alert) => Some(alert),
			_ => None,
		}
	}
}

pub trait ReceiveDrain<M> where M: Send + Clone {
	fn drain(&self) -> Vec<M>;
	fn purge(&self);
}

struct Filter<M> where M: Send {
	accept: Box<Fn(&M) -> bool>,
	sender: Sender<M>,
}

pub struct PubSub<M = Message> where M: Send {
	filters: Vec<Filter<M>>,
}

pub trait Outbox<M = Message> {
	fn post(&self, message: M);
}

pub trait Whiteboard<M = Message> where M: Send + Clone {
	fn subscribe(&mut self, accept: Box<Fn(&M) -> bool>) -> Inbox<M>;
}

pub struct Inbox<M = Message> where M: Send + Clone {
	receiver: Receiver<M>,
}

impl<M> Outbox<M> for PubSub<M> where M: Send + Clone {
	fn post(&self, message: M) {
		for filter in &self.filters {
			if (*filter.accept)(&message) {
				filter.sender.send(message.clone()).is_ok();
			}
		}
	}
}

impl<M> PubSub<M> where M: Send + Clone {
	pub fn new() -> Self {
		PubSub {
			filters: Vec::new(),
		}
	}
}

impl<M> Whiteboard<M> for PubSub<M> where M: Send + Clone {
	fn subscribe(&mut self, accept: Box<Fn(&M) -> bool>) -> Inbox<M> {
		let (sender, receiver) = mpsc::channel();
		self.filters.push(Filter {
			accept,
			sender,
		});
		Inbox { receiver }
	}
}

impl<M> ReceiveDrain<M> for Inbox<M> where M: Send + Clone {
	fn drain(&self) -> Vec<M> {
		let mut acc = Vec::new();
		'out: loop {
			match self.receiver.try_recv() {
				Ok(received) => { acc.push(received); }
				Err(_) => { break 'out; }
			}
		}
		acc
	}

	fn purge(&self) {
		'out: loop {
			if self.receiver.try_recv().is_err() {
				break 'out;
			}
		}
	}
}

pub trait DrainInto<M, T>: ReceiveDrain<M> where M: Send + Clone + Into<Option<T>> {
	fn drain_into(&self) -> Vec<T> {
		self.drain().into_iter().map(|i| i.into()).filter_map(|i| i).collect::<Vec<T>>()
	}
}
