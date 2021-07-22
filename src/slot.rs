#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ArenaSlotState<T> {
	Free,
	Occupied {
		data: T,
		previous_occupied_slot_index: Option<usize>,
		next_occupied_slot_index: Option<usize>,
	},
}

#[derive(Debug)]
pub(crate) struct ArenaSlot<T> {
	pub(crate) state: ArenaSlotState<T>,
	pub(crate) generation: usize,
}

impl<T> ArenaSlot<T> {
	pub(crate) fn new() -> Self {
		Self {
			state: ArenaSlotState::Free,
			generation: 0,
		}
	}

	#[cfg(test)]
	pub(crate) fn is_free(&self) -> bool {
		if let ArenaSlotState::Free = &self.state {
			true
		} else {
			false
		}
	}

	#[cfg(test)]
	pub(crate) fn previous_occupied_slot_index(&self) -> Option<usize> {
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
	pub(crate) fn next_occupied_slot_index(&self) -> Option<usize> {
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

	pub(crate) fn set_previous_occupied_slot_index(&mut self, index: Option<usize>) {
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

	pub(crate) fn set_next_occupied_slot_index(&mut self, index: Option<usize>) {
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
	pub(crate) fn is_occupied_with_data(&self, intended_data: T) -> bool {
		if let ArenaSlotState::Occupied { data, .. } = &self.state {
			*data == intended_data
		} else {
			false
		}
	}
}
