#[cfg(test)]
mod test;

use std::{
	error::Error,
	fmt::Display,
	iter::Enumerate,
	sync::{
		atomic::{AtomicBool, AtomicUsize, Ordering},
		Arc,
	},
};

const NO_NEXT_FREE_SLOT: usize = usize::MAX;

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
	next_free_slot_index: AtomicUsize,
}

#[derive(Debug)]
struct ControllerInner {
	slots: Vec<ControllerSlot>,
	first_free_slot_index: AtomicUsize,
}

impl ControllerInner {
	fn new(capacity: usize) -> Self {
		Self {
			slots: (0..capacity)
				.map(|i| ControllerSlot {
					free: AtomicBool::new(true),
					generation: AtomicUsize::new(0),
					next_free_slot_index: AtomicUsize::new(if i < capacity - 1 {
						i + 1
					} else {
						NO_NEXT_FREE_SLOT
					}),
				})
				.collect(),
			first_free_slot_index: AtomicUsize::new(0),
		}
	}

	fn try_reserve(&self) -> Result<Index, ArenaFull> {
		let first_free_slot_index = self.first_free_slot_index.load(Ordering::SeqCst);
		if first_free_slot_index == NO_NEXT_FREE_SLOT {
			return Err(ArenaFull);
		}
		let slot = &self.slots[first_free_slot_index];
		slot.free.store(false, Ordering::SeqCst);
		self.first_free_slot_index.store(
			slot.next_free_slot_index.load(Ordering::SeqCst),
			Ordering::SeqCst,
		);
		Ok(Index {
			index: first_free_slot_index,
			generation: slot.generation.load(Ordering::SeqCst),
		})
	}

	fn free(&self, index: usize) {
		let slot = &self.slots[index];
		slot.free.store(true, Ordering::SeqCst);
		slot.generation.fetch_add(1, Ordering::SeqCst);
		slot.next_free_slot_index.store(
			self.first_free_slot_index.load(Ordering::SeqCst),
			Ordering::SeqCst,
		);
		self.first_free_slot_index.store(index, Ordering::SeqCst);
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

	pub fn insert_with_index(&mut self, index: Index, data: T) -> Result<(), IndexNotReserved> {
		let slot = &mut self.slots[index.index];
		if slot.data.is_some() || slot.generation != index.generation {
			return Err(IndexNotReserved);
		}
		slot.data = Some(data);
		Ok(())
	}

	pub fn insert(&mut self, data: T) -> Result<Index, ArenaFull> {
		let index = self.controller.try_reserve()?;
		self.insert_with_index(index, data).unwrap();
		Ok(index)
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

	pub fn iter(&self) -> Iter<T> {
		Iter::new(self)
	}

	pub fn iter_mut(&mut self) -> IterMut<T> {
		IterMut::new(self)
	}
}

pub struct Iter<'a, T> {
	slot_iter: Enumerate<std::slice::Iter<'a, ArenaSlot<T>>>,
}

impl<'a, T> Iter<'a, T> {
	fn new(arena: &'a Arena<T>) -> Self {
		Self {
			slot_iter: arena.slots.iter().enumerate(),
		}
	}
}

impl<'a, T> Iterator for Iter<'a, T> {
	type Item = (Index, &'a T);

	fn next(&mut self) -> Option<Self::Item> {
		while let Some((i, slot)) = self.slot_iter.next() {
			if let Some(data) = &slot.data {
				return Some((
					Index {
						index: i,
						generation: slot.generation,
					},
					data,
				));
			}
		}
		None
	}
}

pub struct IterMut<'a, T> {
	slot_iter: Enumerate<std::slice::IterMut<'a, ArenaSlot<T>>>,
}

impl<'a, T> IterMut<'a, T> {
	fn new(arena: &'a mut Arena<T>) -> Self {
		Self {
			slot_iter: arena.slots.iter_mut().enumerate(),
		}
	}
}

impl<'a, T> Iterator for IterMut<'a, T> {
	type Item = (Index, &'a mut T);

	fn next(&mut self) -> Option<Self::Item> {
		while let Some((i, slot)) = self.slot_iter.next() {
			if let Some(data) = &mut slot.data {
				return Some((
					Index {
						index: i,
						generation: slot.generation,
					},
					data,
				));
			}
		}
		None
	}
}

impl<'a, T> IntoIterator for &'a Arena<T> {
	type Item = (Index, &'a T);

	type IntoIter = Iter<'a, T>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl<'a, T> IntoIterator for &'a mut Arena<T> {
	type Item = (Index, &'a mut T);

	type IntoIter = IterMut<'a, T>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter_mut()
	}
}
