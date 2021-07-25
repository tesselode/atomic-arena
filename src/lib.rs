/*!
`atomic_arena` provides a generational [`Arena`] that you can reserve
an [`Index`] for ahead of time using a [`Controller`]. [`Controller`]s
are backed by atomics, so they can be cloned and used across threads
and still have consistent state.

This is useful when you want to insert an item into an [`Arena`] on
a different thread, but you want to have a valid [`Index`] for that
item immediately on the current thread.
*/

mod controller;
pub mod error;
mod slot;

#[cfg(test)]
mod test;

pub use controller::Controller;

use error::{ArenaFull, IndexNotReserved};
use slot::{ArenaSlot, ArenaSlotState};

/// A unique identifier for an item in an [`Arena`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Index {
	index: usize,
	generation: usize,
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

	fn remove_at_raw_index(&mut self, index: usize) -> Option<T> {
		let slot = &mut self.slots[index];
		let state = std::mem::replace(&mut slot.state, ArenaSlotState::Free);
		match state {
			ArenaSlotState::Free => None,
			ArenaSlotState::Occupied {
				data,
				previous_occupied_slot_index,
				next_occupied_slot_index,
			} => {
				slot.generation += 1;
				self.controller.free(index);

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
				if self.first_occupied_slot_index.unwrap() == index {
					self.first_occupied_slot_index = next_occupied_slot_index;
				}

				Some(data)
			}
		}
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
		self.remove_at_raw_index(index.index)
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

	/// Retains only the elements specified by the predicate.
	///
	/// In other words, remove all elements e such that f(&e) returns false.
	pub fn retain(&mut self, mut f: impl FnMut(&T) -> bool) {
		let mut index = match self.first_occupied_slot_index {
			Some(index) => index,
			None => return,
		};
		loop {
			if let ArenaSlotState::Occupied {
				data,
				next_occupied_slot_index,
				..
			} = &self.slots[index].state
			{
				let next_occupied_slot_index = match next_occupied_slot_index {
					Some(index) => Some(*index),
					None => None,
				};
				if !f(data) {
					self.remove_at_raw_index(index);
				}
				index = match next_occupied_slot_index {
					Some(index) => index,
					None => return,
				}
			} else {
				panic!("expected the slot pointed to by first_occupied_slot_index/next_occupied_slot_index to be occupied")
			}
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

	/// Returns an iterator that removes and yields all elements
	/// for which `filter(&element)` returns `true`.
	pub fn drain_filter<F: FnMut(&T) -> bool>(&mut self, filter: F) -> DrainFilter<T, F> {
		DrainFilter::new(self, filter)
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

/// An iterator that removes and yields elements from an
/// [`Arena`] according to a filter function.
pub struct DrainFilter<'a, T, F: FnMut(&T) -> bool> {
	arena: &'a mut Arena<T>,
	filter: F,
	next_occupied_slot_index: Option<usize>,
}

impl<'a, T, F: FnMut(&T) -> bool> DrainFilter<'a, T, F> {
	fn new(arena: &'a mut Arena<T>, filter: F) -> Self {
		Self {
			next_occupied_slot_index: arena.first_occupied_slot_index,
			arena,
			filter,
		}
	}
}

impl<'a, T, F: FnMut(&T) -> bool> Iterator for DrainFilter<'a, T, F> {
	type Item = (Index, T);

	fn next(&mut self) -> Option<Self::Item> {
		while let Some(raw_index) = self.next_occupied_slot_index {
			let slot = &mut self.arena.slots[raw_index];
			if let ArenaSlotState::Occupied {
				data,
				next_occupied_slot_index,
				..
			} = &mut slot.state
			{
				self.next_occupied_slot_index = *next_occupied_slot_index;
				if (self.filter)(&data) {
					let index = Index {
						index: raw_index,
						generation: slot.generation,
					};
					return self
						.arena
						.remove_at_raw_index(raw_index)
						.map(|element| (index, element));
				}
			} else {
				panic!("the iterator should not encounter a free slot");
			}
		}
		None
	}
}
