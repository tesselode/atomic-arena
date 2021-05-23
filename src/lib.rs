use std::sync::{
	atomic::{AtomicBool, AtomicUsize, Ordering},
	Arc,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Index {
	index: usize,
	generation: usize,
}

#[derive(Debug)]
struct ControllerSlot {
	free: AtomicBool,
	generation: AtomicUsize,
}

impl ControllerSlot {
	pub fn new() -> Self {
		Self {
			free: AtomicBool::new(true),
			generation: AtomicUsize::new(0),
		}
	}
}

#[derive(Debug)]
struct ControllerInner {
	slots: Vec<ControllerSlot>,
}

impl ControllerInner {
	pub fn new(capacity: usize) -> Self {
		Self {
			slots: (0..capacity).map(|_| ControllerSlot::new()).collect(),
		}
	}

	pub fn try_reserve(&self) -> Option<Index> {
		for (i, slot) in self.slots.iter().enumerate() {
			if slot.free.load(Ordering::SeqCst) {
				slot.free.store(false, Ordering::SeqCst);
				return Some(Index {
					index: i,
					generation: slot.generation.load(Ordering::SeqCst),
				});
			}
		}
		None
	}
}

#[derive(Debug, Clone)]
pub struct Controller(Arc<ControllerInner>);

impl Controller {
	pub fn new(capacity: usize) -> Self {
		Self(Arc::new(ControllerInner::new(capacity)))
	}

	pub fn try_reserve(&self) -> Option<Index> {
		self.0.try_reserve()
	}
}

#[derive(Debug)]
struct ArenaSlot<T> {
	data: Option<T>,
	generation: usize,
}

impl<T> ArenaSlot<T> {
	pub fn new() -> Self {
		Self {
			data: None,
			generation: 0,
		}
	}
}

#[derive(Debug)]
pub struct Arena<T> {
	slots: Vec<ArenaSlot<T>>,
	controller: Controller,
}

impl<T> Arena<T> {
	pub fn new(capacity: usize) -> Self {
		Self {
			slots: (0..capacity).map(|_| ArenaSlot::new()).collect(),
			controller: Controller::new(capacity),
		}
	}

	pub fn controller(&self) -> Controller {
		self.controller.clone()
	}
}

#[cfg(test)]
mod test {
	use crate::{Arena, Index};

	#[test]
	fn reserve() {
		let arena = Arena::<()>::new(3);
		let controller = arena.controller();
		assert_eq!(
			controller.try_reserve(),
			Some(Index {
				index: 0,
				generation: 0,
			})
		);
		assert_eq!(
			controller.try_reserve(),
			Some(Index {
				index: 1,
				generation: 0,
			})
		);
		assert_eq!(
			controller.try_reserve(),
			Some(Index {
				index: 2,
				generation: 0,
			})
		);
		assert_eq!(controller.try_reserve(), None);
	}
}
