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

/// Represents that a [`ControllerSlot`] does not have a free slot
/// after it.
///
/// This is used because the next free slot variable is an
/// [`AtomicUsize`], but we still need some way to represent the
/// absence of a next free slot.
const NO_NEXT_FREE_SLOT: usize = usize::MAX;

/// An error that's returned when trying to reserve an index
/// on an [`Arena`] that's full.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArenaFull;

impl Display for ArenaFull {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("Cannot reserve an index because the arena is full")
	}
}

impl Error for ArenaFull {}

/// An error that's returned when trying to insert into an
/// [`Arena`] with an index that hasn't been reserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IndexNotReserved;

impl Display for IndexNotReserved {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("Cannot insert with this index because it is not reserved")
	}
}

impl Error for IndexNotReserved {}

/// A unique identifier for an item in an [`Arena`].
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

/// The shared state for all [`Controller`]s for an [`Arena`].
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

/// Manages [`Index`] reservations for an [`Arena`].
#[derive(Debug, Clone)]
pub struct Controller(Arc<ControllerInner>);

impl Controller {
	fn new(capacity: usize) -> Self {
		Self(Arc::new(ControllerInner::new(capacity)))
	}

	/// Tries to reserve an index for the [`Arena`].
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

/// A container of items that can be accessed via an [`Index`].
#[derive(Debug)]
pub struct Arena<T> {
	controller: Controller,
	slots: Vec<ArenaSlot<T>>,
	first_occupied_slot_index: Option<usize>,
}

impl<T> Arena<T> {
	/// Creates a new [`Arena`] with enough space for `capacity`
	/// number of items.
	pub fn new(capacity: usize) -> Self {
		Self {
			controller: Controller::new(capacity),
			slots: (0..capacity).map(|_| ArenaSlot::new()).collect(),
			first_occupied_slot_index: None,
		}
	}

	/// Returns a [`Controller`] for this [`Arena`].
	pub fn controller(&self) -> Controller {
		self.controller.clone()
	}

	/// Returns the total capacity for this [`Arena`].
	pub fn capacity(&self) -> usize {
		self.slots.len()
	}

	/// Returns the number of items currently in the [`Arena`].
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

	/// Tries to insert an item into the [`Arena`] with a previously
	/// reserved [`Index`].
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

	/// Tries to reserve an [`Index`], and, if successful, inserts
	/// an item into the [`Arena`] with that [`Index`] and
	/// returns the [`Index`].
	pub fn insert(&mut self, data: T) -> Result<Index, ArenaFull> {
		let index = self.controller.try_reserve()?;
		self.insert_with_index(index, data).unwrap();
		Ok(index)
	}

	/// If the [`Arena`] contains an item with the given [`Index`],
	/// removes it from the [`Arena`] and returns `Some(item)`.
	/// Otherwise, returns `None`.
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

				// update the head if needed.
				//
				// `first_occupied_slot_index` should always be `Some` in this case,
				// because this branch of the `match` is only taken if the slot is
				// occupied, and if any slots are occupied, `first_occupied_slot_index`
				// should be `Some`. if not, there's a major bug that needs addressing.
				if self.first_occupied_slot_index.unwrap() == index.index {
					self.first_occupied_slot_index = next_occupied_slot_index;
				}

				Some(data)
			}
		}
	}

	/// Returns a shared reference to the item in the [`Arena`] with
	/// the given [`Index`] if it exists. Otherwise, returns `None`.
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

	/// Returns a mutable reference to the item in the [`Arena`] with
	/// the given [`Index`] if it exists. Otherwise, returns `None`.
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

	/// Returns an iterator over shared references to the items in
	/// the [`Arena`].
	///
	/// The most recently added items will be visited first.
	pub fn iter(&self) -> Iter<T> {
		Iter::new(self)
	}

	/// Returns an iterator over mutable references to the items in
	/// the [`Arena`].
	///
	/// The most recently added items will be visited first.
	pub fn iter_mut(&mut self) -> IterMut<T> {
		IterMut::new(self)
	}
}

/// Iterates over shared references to the items in
/// the [`Arena`].
///
/// The most recently added items will be visited first.
pub struct Iter<'a, T> {
	next_occupied_slot_index: Option<usize>,
	arena: &'a Arena<T>,
}

impl<'a, T> Iter<'a, T> {
	fn new(arena: &'a Arena<T>) -> Self {
		Self {
			next_occupied_slot_index: arena.first_occupied_slot_index,
			arena,
		}
	}
}

impl<'a, T> Iterator for Iter<'a, T> {
	type Item = (Index, &'a T);

	fn next(&mut self) -> Option<Self::Item> {
		if let Some(index) = self.next_occupied_slot_index {
			let slot = &self.arena.slots[index];
			if let ArenaSlotState::Occupied {
				data,
				next_occupied_slot_index,
				..
			} = &slot.state
			{
				self.next_occupied_slot_index = *next_occupied_slot_index;
				Some((
					Index {
						index,
						generation: slot.generation,
					},
					data,
				))
			} else {
				panic!("the iterator should not encounter a free slot");
			}
		} else {
			None
		}
	}
}

/// Iterates over mutable references to the items in
/// the [`Arena`].
///
/// The most recently added items will be visited first.
pub struct IterMut<'a, T> {
	next_occupied_slot_index: Option<usize>,
	arena: &'a mut Arena<T>,
}

impl<'a, T> IterMut<'a, T> {
	fn new(arena: &'a mut Arena<T>) -> Self {
		Self {
			next_occupied_slot_index: arena.first_occupied_slot_index,
			arena,
		}
	}
}

impl<'a, T> Iterator for IterMut<'a, T> {
	type Item = (Index, &'a mut T);

	fn next(&mut self) -> Option<Self::Item> {
		if let Some(index) = self.next_occupied_slot_index {
			let slot = &mut self.arena.slots[index];
			if let ArenaSlotState::Occupied {
				data,
				next_occupied_slot_index,
				..
			} = &mut slot.state
			{
				self.next_occupied_slot_index = *next_occupied_slot_index;
				Some((
					Index {
						index,
						generation: slot.generation,
					},
					// using a small bit of unsafe code here to get around
					// borrow checker limitations. this workaround is stolen
					// from slotmap: https://github.com/orlp/slotmap/blob/master/src/hop.rs#L1165
					unsafe {
						let data: *mut T = &mut *data;
						&mut *data
					},
				))
			} else {
				panic!("the iterator should not encounter a free slot");
			}
		} else {
			None
		}
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
