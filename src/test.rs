use crate::{
	error::{ArenaFull, IndexNotReserved},
	Arena,
};

#[test]
fn try_reserve() {
	let arena = Arena::<()>::new(3);
	let controller = arena.controller();
	// we should be able to reserve 3 indices
	// because the capacity is 3
	assert!(controller.try_reserve().is_ok());
	assert!(controller.try_reserve().is_ok());
	assert!(controller.try_reserve().is_ok());
	// we should not be able to reserve a 4th index
	assert_eq!(controller.try_reserve(), Err(ArenaFull));
}

#[test]
fn insert_with_index() {
	let mut arena = Arena::new(3);
	let controller = arena.controller();
	let index = controller.try_reserve().unwrap();
	// we should be able to insert with the index we reserved
	assert!(arena.insert_with_index(index, 1).is_ok());
	// the item should be in the arena
	assert_eq!(arena.get(index), Some(&1));
	// we should not be able to insert again with the same index
	assert_eq!(arena.insert_with_index(index, 2), Err(IndexNotReserved));
}

#[test]
fn insert() {
	let mut arena = Arena::new(3);
	// we should be able to insert 3 items
	let index1 = arena.insert(1).unwrap();
	let index2 = arena.insert(2).unwrap();
	let index3 = arena.insert(3).unwrap();
	// we should be able to retrieve those items with the
	// returned indices
	assert_eq!(arena.get(index1), Some(&1));
	assert_eq!(arena.get(index2), Some(&2));
	assert_eq!(arena.get(index3), Some(&3));
	// we should not be able to insert a 4th item
	assert_eq!(arena.insert(4), Err(ArenaFull));
}

#[test]
fn remove() {
	let mut arena = Arena::new(3);
	let index1 = arena.insert(1).unwrap();
	let index2 = arena.insert(2).unwrap();
	let index3 = arena.insert(3).unwrap();
	// we should be able to remove an item and get it back
	assert_eq!(arena.remove(index2), Some(2));
	// if there's no item associated with the index,
	// `remove` should return `None`
	assert_eq!(arena.remove(index2), None);
	// the other items should still be in the arena
	assert_eq!(arena.get(index1), Some(&1));
	assert_eq!(arena.get(index3), Some(&3));
	// there should be space to insert another item now
	assert!(arena.insert(4).is_ok());
}
