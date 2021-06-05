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

#[derive(Debug, Clone, PartialEq, Eq)]
enum ArenaSlotState<T> {
	Free,
	Occupied {
		data: T,
		previous_occupied_slot_index: Option<usize>,
		next_occupied_slot_index: Option<usize>,
	},
}

#[derive(Debug)]
struct ArenaSlot<T> {
	state: ArenaSlotState<T>,
	generation: usize,
}

impl<T> ArenaSlot<T> {
	pub fn new() -> Self {
		Self {
			state: ArenaSlotState::Free,
			generation: 0,
		}
	}

	#[cfg(test)]
	fn is_free(&self) -> bool {
		if let ArenaSlotState::Free = &self.state {
			true
		} else {
			false
		}
	}

	#[cfg(test)]
	fn previous_occupied_slot_index(&self) -> Option<usize> {
		if let ArenaSlotState::Occupied {
			previous_occupied_slot_index,
			..
		} = &self.state
		{
			*previous_occupied_slot_index
		} else {
			None
		}
	}

	#[cfg(test)]
	fn next_occupied_slot_index(&self) -> Option<usize> {
		if let ArenaSlotState::Occupied {
			next_occupied_slot_index,
			..
		} = &self.state
		{
			*next_occupied_slot_index
		} else {
			None
		}
	}

	fn set_previous_occupied_slot_index(&mut self, index: Option<usize>) {
		if let ArenaSlotState::Occupied {
			previous_occupied_slot_index,
			..
		} = &mut self.state
		{
			*previous_occupied_slot_index = index;
		} else {
			panic!("expected a slot to be occupied, but it was not");
		}
	}

	fn set_next_occupied_slot_index(&mut self, index: Option<usize>) {
		if let ArenaSlotState::Occupied {
			next_occupied_slot_index,
			..
		} = &mut self.state
		{
			*next_occupied_slot_index = index;
		} else {
			panic!("expected a slot to be occupied, but it was not");
		}
	}
}

impl<T: PartialEq> ArenaSlot<T> {
	#[cfg(test)]
	fn is_occupied_with_data(&self, intended_data: T) -> bool {
		if let ArenaSlotState::Occupied { data, .. } = &self.state {
			*data == intended_data
		} else {
			false
		}
	}
}

#[derive(Debug)]
pub struct Arena<T> {
	controller: Controller,
	slots: Vec<ArenaSlot<T>>,
	first_occupied_slot_index: Option<usize>,
}

impl<T> Arena<T> {
	pub fn new(capacity: usize) -> Self {
		Self {
			controller: Controller::new(capacity),
			slots: (0..capacity).map(|_| ArenaSlot::new()).collect(),
			first_occupied_slot_index: None,
		}
	}

	pub fn controller(&self) -> Controller {
		self.controller.clone()
	}

	pub fn capacity(&self) -> usize {
		self.slots.len()
	}

	pub fn len(&self) -> usize {
		self.slots
			.iter()
			.filter(|slot| {
				if let ArenaSlotState::Occupied { .. } = &slot.state {
					true
				} else {
					false
				}
			})
			.count()
	}

	pub fn insert_with_index(&mut self, index: Index, data: T) -> Result<(), IndexNotReserved> {
		// make sure the index is reserved
		{
			let slot = &mut self.slots[index.index];
			if let ArenaSlotState::Occupied { .. } = &slot.state {
				return Err(IndexNotReserved);
			}
			if slot.generation != index.generation {
				return Err(IndexNotReserved);
			}
		}

		// update the previous head to point to the new head
		// as the previous occupied slot
		if let Some(head_index) = self.first_occupied_slot_index {
			self.slots[head_index].set_previous_occupied_slot_index(Some(index.index));
		}

		// insert the new data
		self.slots[index.index].state = ArenaSlotState::Occupied {
			data,
			previous_occupied_slot_index: None,
			next_occupied_slot_index: self.first_occupied_slot_index,
		};

		// update the head
		self.first_occupied_slot_index = Some(index.index);

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
		if slot.generation != index.generation {
			return None;
		}
		let state = std::mem::replace(&mut slot.state, ArenaSlotState::Free);
		match state {
			ArenaSlotState::Free => None,
			ArenaSlotState::Occupied {
				data,
				previous_occupied_slot_index,
				next_occupied_slot_index,
			} => {
				slot.generation += 1;
				self.controller.free(index.index);

				// update the pointers of the previous and next slots
				if let Some(previous_index) = previous_occupied_slot_index {
					self.slots[previous_index]
						.set_next_occupied_slot_index(next_occupied_slot_index);
				}
				if let Some(next_index) = next_occupied_slot_index {
					self.slots[next_index]
						.set_previous_occupied_slot_index(previous_occupied_slot_index);
				}

				// update the head if needed
				if self.first_occupied_slot_index.unwrap() == index.index {
					self.first_occupied_slot_index = next_occupied_slot_index;
				}

				Some(data)
			}
		}
	}

	pub fn get(&self, index: Index) -> Option<&T> {
		let slot = &self.slots[index.index];
		if slot.generation != index.generation {
			return None;
		}
		match &slot.state {
			ArenaSlotState::Free => None,
			ArenaSlotState::Occupied { data, .. } => Some(data),
		}
	}

	pub fn get_mut(&mut self, index: Index) -> Option<&mut T> {
		let slot = &mut self.slots[index.index];
		if slot.generation != index.generation {
			return None;
		}
		match &mut slot.state {
			ArenaSlotState::Free => None,
			ArenaSlotState::Occupied { data, .. } => Some(data),
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
			if let ArenaSlotState::Occupied { data, .. } = &slot.state {
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
			if let ArenaSlotState::Occupied { data, .. } = &mut slot.state {
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
