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

#[test]
fn remove() {
	let mut arena = Arena::new(3);
	let controller = arena.controller();
	let index1 = controller.try_reserve().unwrap();
	let index2 = controller.try_reserve().unwrap();
	let index3 = controller.try_reserve().unwrap();
	arena.insert(index1, 1).unwrap();
	arena.insert(index2, 2).unwrap();
	arena.insert(index3, 3).unwrap();
	assert_eq!(arena.remove(index1), Some(1));
	assert_eq!(arena.remove(index3), Some(3));
	assert_eq!(arena.remove(index1), None);
	assert_eq!(arena.slots[0].data, None);
	assert_eq!(arena.slots[0].generation, 1);
	assert_eq!(arena.slots[1].data, Some(2));
	assert_eq!(arena.slots[1].generation, 0);
	assert_eq!(arena.slots[2].data, None);
	assert_eq!(arena.slots[2].generation, 1);
	let index4 = controller.try_reserve();
	assert_eq!(
		index4,
		Ok(Index {
			index: 0,
			generation: 1,
		})
	);
	let index4 = index4.unwrap();
	assert!(arena.insert(index4, 4).is_ok());
	assert_eq!(arena.slots[0].data, Some(4));
	assert_eq!(arena.slots[0].generation, 1);
	assert_eq!(arena.slots[1].data, Some(2));
	assert_eq!(arena.slots[1].generation, 0);
	assert_eq!(arena.slots[2].data, None);
	assert_eq!(arena.slots[2].generation, 1);
}

#[test]
fn get() {
	let mut arena = Arena::new(3);
	let controller = arena.controller();
	let index1 = controller.try_reserve().unwrap();
	let index2 = controller.try_reserve().unwrap();
	let index3 = controller.try_reserve().unwrap();
	arena.insert(index1, 1).unwrap();
	arena.insert(index2, 2).unwrap();
	arena.insert(index3, 3).unwrap();
	assert_eq!(arena.get(index1), Some(&1));
	assert_eq!(arena.get(index2), Some(&2));
	assert_eq!(arena.get(index3), Some(&3));
	assert_eq!(arena.get_mut(index1), Some(&mut 1));
	assert_eq!(arena.get_mut(index2), Some(&mut 2));
	assert_eq!(arena.get_mut(index3), Some(&mut 3));
	arena.remove(index1);
	assert_eq!(arena.get(index1), None);
	assert_eq!(arena.get(index2), Some(&2));
	assert_eq!(arena.get(index3), Some(&3));
	assert_eq!(arena.get_mut(index1), None);
	assert_eq!(arena.get_mut(index2), Some(&mut 2));
	assert_eq!(arena.get_mut(index3), Some(&mut 3));
}

#[test]
fn len() {
	let mut arena = Arena::new(3);
	let controller = arena.controller();
	let index1 = controller.try_reserve().unwrap();
	let index2 = controller.try_reserve().unwrap();
	let index3 = controller.try_reserve().unwrap();
	assert_eq!(arena.len(), 0);
	arena.insert(index1, 1).unwrap();
	assert_eq!(arena.len(), 1);
	arena.insert(index2, 2).unwrap();
	assert_eq!(arena.len(), 2);
	arena.insert(index3, 3).unwrap();
	assert_eq!(arena.len(), 3);
	arena.remove(index1);
	assert_eq!(arena.len(), 2);
}

#[test]
fn capacity() {
	let mut arena = Arena::new(3);
	let controller = arena.controller();
	let index1 = controller.try_reserve().unwrap();
	let index2 = controller.try_reserve().unwrap();
	let index3 = controller.try_reserve().unwrap();
	assert_eq!(arena.capacity(), 3);
	arena.insert(index1, 1).unwrap();
	assert_eq!(arena.capacity(), 3);
	arena.insert(index2, 2).unwrap();
	assert_eq!(arena.capacity(), 3);
	arena.insert(index3, 3).unwrap();
	assert_eq!(arena.capacity(), 3);
	arena.remove(index1);
	assert_eq!(arena.capacity(), 3);
}
