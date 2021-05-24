#[cfg(test)]
mod test;

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
	fn new() -> Self {
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
	fn new(capacity: usize) -> Self {
		Self {
			slots: (0..capacity).map(|_| ControllerSlot::new()).collect(),
		}
	}

	fn try_reserve(&self) -> Result<Index, ArenaFull> {
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

	fn free(&self, index: usize) {
		let slot = &self.slots[index];
		slot.free.store(true, Ordering::SeqCst);
		slot.generation.fetch_add(1, Ordering::SeqCst);
	}
}

#[derive(Debug, Clone)]
pub struct Controller(Arc<ControllerInner>);

impl Controller {
	fn new(capacity: usize) -> Self {
		Self(Arc::new(ControllerInner::new(capacity)))
	}

	pub fn try_reserve(&self) -> Result<Index, ArenaFull> {
		self.0.try_reserve()
	}

	fn free(&self, index: usize) {
		self.0.free(index);
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

	pub fn capacity(&self) -> usize {
		self.slots.len()
	}

	pub fn len(&self) -> usize {
		self.slots.iter().filter(|slot| slot.data.is_some()).count()
	}

	pub fn insert(&mut self, index: Index, data: T) -> Result<(), IndexNotReserved> {
		let slot = &mut self.slots[index.index];
		if slot.data.is_some() || slot.generation != index.generation {
			return Err(IndexNotReserved);
		}
		slot.data = Some(data);
		Ok(())
	}

	pub fn remove(&mut self, index: Index) -> Option<T> {
		// TODO: answer the following questions:
		// - if you reserve a key, then try to remove the key
		// without having inserted anything, should the slot
		// be unreserved? the current answer is no
		// - what should happen if you try to remove a slot
		// with the wrong generation? currently the answer is
		// it just returns None like normal
		let slot = &mut self.slots[index.index];
		if slot.data.is_none() || slot.generation != index.generation {
			return None;
		}
		slot.generation += 1;
		self.controller.free(index.index);
		slot.data.take()
	}

	pub fn get(&self, index: Index) -> Option<&T> {
		let slot = &self.slots[index.index];
		if slot.generation == index.generation {
			slot.data.as_ref()
		} else {
			None
		}
	}

	pub fn get_mut(&mut self, index: Index) -> Option<&mut T> {
		let slot = &mut self.slots[index.index];
		if slot.generation == index.generation {
			slot.data.as_mut()
		} else {
			None
		}
	}
}
