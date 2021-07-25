use crate::{
	error::{ArenaFull, IndexNotReserved},
	Arena,
};

#[test]
fn controller() {
	let arena = Arena::<()>::new(1);
	let controller1 = arena.controller();
	controller1.try_reserve().unwrap();
	// controllers should share state
	let controller2 = arena.controller();
	assert_eq!(controller2.try_reserve(), Err(ArenaFull));
}

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
fn capacity() {
	let mut arena = Arena::new(3);
	assert_eq!(arena.capacity(), 3);
	// the capacity of the arena should be constant
	arena.insert(1).unwrap();
	assert_eq!(arena.capacity(), 3);
	let index2 = arena.insert(2).unwrap();
	assert_eq!(arena.capacity(), 3);
	arena.remove(index2);
	assert_eq!(arena.capacity(), 3);
}

#[test]
fn len() {
	let mut arena = Arena::new(3);
	arena.insert(1).unwrap();
	assert_eq!(arena.len(), 1);
	arena.insert(2).unwrap();
	assert_eq!(arena.len(), 2);
	let index3 = arena.insert(3).unwrap();
	assert_eq!(arena.len(), 3);
	// if inserting an element fails, the length should not
	// increase
	arena.insert(4).ok();
	assert_eq!(arena.len(), 3);
	arena.remove(index3);
	assert_eq!(arena.len(), 2);
	// if removing an element fails, the length should not
	// decrease
	arena.remove(index3);
	assert_eq!(arena.len(), 2);
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
	// we shouldn't be able to remove the new item in the
	// same slot with an old index
	assert_eq!(arena.remove(index2), None);
}

#[test]
fn get() {
	let mut arena = Arena::new(3);
	let index1 = arena.insert(1).unwrap();
	let index2 = arena.insert(2).unwrap();
	let index3 = arena.insert(3).unwrap();
	// get should return shared references
	assert_eq!(arena.get(index1), Some(&1));
	assert_eq!(arena.get(index2), Some(&2));
	assert_eq!(arena.get(index3), Some(&3));
	// get_mut should return mutable references
	assert_eq!(arena.get_mut(index1), Some(&mut 1));
	assert_eq!(arena.get_mut(index2), Some(&mut 2));
	assert_eq!(arena.get_mut(index3), Some(&mut 3));
	// after removing an item, get should return None
	arena.remove(index2);
	assert_eq!(arena.get(index2), None);
	// even after inserting a new item into the same slot,
	// the old index shouldn't work
	arena.insert(4).unwrap();
	assert_eq!(arena.get(index2), None);
}

#[test]
fn retain() {
	let mut arena = Arena::new(6);
	let index1 = arena.insert(1).unwrap();
	let index2 = arena.insert(2).unwrap();
	let index3 = arena.insert(3).unwrap();
	let index4 = arena.insert(4).unwrap();
	let index5 = arena.insert(5).unwrap();
	let index6 = arena.insert(6).unwrap();
	arena.retain(|num| num % 2 == 0);
	assert_eq!(arena.get(index1), None);
	assert_eq!(arena.get(index2), Some(&2));
	assert_eq!(arena.get(index3), None);
	assert_eq!(arena.get(index4), Some(&4));
	assert_eq!(arena.get(index5), None);
	assert_eq!(arena.get(index6), Some(&6));
}

#[test]
fn iter() {
	let mut arena = Arena::new(3);
	let index1 = arena.insert(1).unwrap();
	let index2 = arena.insert(2).unwrap();
	let index3 = arena.insert(3).unwrap();
	// iterators should visit all values
	let mut iter = arena.iter();
	assert_eq!(iter.next(), Some((index3, &3)));
	assert_eq!(iter.next(), Some((index2, &2)));
	assert_eq!(iter.next(), Some((index1, &1)));
	assert_eq!(iter.next(), None);
	// iterators should not visit removed values
	arena.remove(index2);
	let mut iter = arena.iter();
	assert_eq!(iter.next(), Some((index3, &3)));
	assert_eq!(iter.next(), Some((index1, &1)));
	assert_eq!(iter.next(), None);
	// iteration should always be newest first
	let index4 = arena.insert(4).unwrap();
	let mut iter = arena.iter();
	assert_eq!(iter.next(), Some((index4, &4)));
	assert_eq!(iter.next(), Some((index3, &3)));
	assert_eq!(iter.next(), Some((index1, &1)));
	assert_eq!(iter.next(), None);
}

#[test]
fn iter_mut() {
	let mut arena = Arena::new(3);
	let index1 = arena.insert(1).unwrap();
	let index2 = arena.insert(2).unwrap();
	let index3 = arena.insert(3).unwrap();
	// iterators should visit all values
	let mut iter = arena.iter_mut();
	assert_eq!(iter.next(), Some((index3, &mut 3)));
	assert_eq!(iter.next(), Some((index2, &mut 2)));
	assert_eq!(iter.next(), Some((index1, &mut 1)));
	assert_eq!(iter.next(), None);
	// iterators should not visit removed values
	arena.remove(index2);
	let mut iter = arena.iter_mut();
	assert_eq!(iter.next(), Some((index3, &mut 3)));
	assert_eq!(iter.next(), Some((index1, &mut 1)));
	assert_eq!(iter.next(), None);
	// iteration should always be newest first
	let index4 = arena.insert(4).unwrap();
	let mut iter = arena.iter_mut();
	assert_eq!(iter.next(), Some((index4, &mut 4)));
	assert_eq!(iter.next(), Some((index3, &mut 3)));
	assert_eq!(iter.next(), Some((index1, &mut 1)));
	assert_eq!(iter.next(), None);
}

#[test]
fn drain_filter() {
	let mut arena = Arena::new(6);
	let index1 = arena.insert(1).unwrap();
	let index2 = arena.insert(2).unwrap();
	let index3 = arena.insert(3).unwrap();
	let index4 = arena.insert(4).unwrap();
	let index5 = arena.insert(5).unwrap();
	let index6 = arena.insert(6).unwrap();
	let mut iter = arena.drain_filter(|num| num % 2 == 0);
	assert_eq!(iter.next(), Some((index6, 6)));
	assert_eq!(iter.next(), Some((index4, 4)));
	assert_eq!(iter.next(), Some((index2, 2)));
	assert_eq!(iter.next(), None);
	assert_eq!(arena.len(), 3);
	assert_eq!(arena.get(index1), Some(&1));
	assert_eq!(arena.get(index2), None);
	assert_eq!(arena.get(index3), Some(&3));
	assert_eq!(arena.get(index4), None);
	assert_eq!(arena.get(index5), Some(&5));
	assert_eq!(arena.get(index6), None);
}
