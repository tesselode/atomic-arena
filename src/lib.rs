use std::{
	error::Error,
	fmt::Display,
	sync::{
		atomic::{AtomicBool, AtomicUsize, Ordering},
		Arc,
	},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArenaFull;

impl Display for ArenaFull {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("Cannot reserve an index because the arena is full")
	}
}

impl Error for ArenaFull {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IndexNotReserved;

impl Display for IndexNotReserved {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("Cannot insert with this index because it is not reserved")
	}
}

impl Error for IndexNotReserved {}

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

	pub fn try_reserve(&self) -> Result<Index, ArenaFull> {
		for (i, slot) in self.slots.iter().enumerate() {
			if slot.free.load(Ordering::SeqCst) {
				slot.free.store(false, Ordering::SeqCst);
				return Ok(Index {
					index: i,
					generation: slot.generation.load(Ordering::SeqCst),
				});
			}
		}
		Err(ArenaFull)
	}
}

#[derive(Debug, Clone)]
pub struct Controller(Arc<ControllerInner>);

impl Controller {
	pub fn new(capacity: usize) -> Self {
		Self(Arc::new(ControllerInner::new(capacity)))
	}

	pub fn try_reserve(&self) -> Result<Index, ArenaFull> {
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

	pub fn insert(&mut self, index: Index, data: T) -> Result<(), IndexNotReserved> {
		let slot = &mut self.slots[index.index];
		if slot.data.is_some() || slot.generation != index.generation {
			return Err(IndexNotReserved);
		}
		slot.data = Some(data);
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::{Arena, ArenaFull, Index, IndexNotReserved};

	#[test]
	fn reserve() {
		let arena = Arena::<()>::new(3);
		let controller = arena.controller();
		assert_eq!(
			controller.try_reserve(),
			Ok(Index {
				index: 0,
				generation: 0,
			})
		);
		assert_eq!(
			controller.try_reserve(),
			Ok(Index {
				index: 1,
				generation: 0,
			})
		);
		assert_eq!(
			controller.try_reserve(),
			Ok(Index {
				index: 2,
				generation: 0,
			})
		);
		assert_eq!(controller.try_reserve(), Err(ArenaFull));
	}

	#[test]
	fn insert() {
		let mut arena = Arena::new(3);
		let controller = arena.controller();
		let index1 = controller.try_reserve().unwrap();
		let index2 = controller.try_reserve().unwrap();
		let index3 = controller.try_reserve().unwrap();
		assert!(arena.insert(index1, 1).is_ok());
		assert!(arena.insert(index2, 2).is_ok());
		assert!(arena.insert(index3, 3).is_ok());
		assert_eq!(arena.slots[0].data, Some(1));
		assert_eq!(arena.slots[0].generation, 0);
		assert_eq!(arena.slots[1].data, Some(2));
		assert_eq!(arena.slots[1].generation, 0);
		assert_eq!(arena.slots[2].data, Some(3));
		assert_eq!(arena.slots[2].generation, 0);
		assert_eq!(arena.insert(index1, 4), Err(IndexNotReserved));
	}
}
