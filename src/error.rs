//! Error types.

use std::{error::Error, fmt::Display};

/// Returned when trying to reserve an key on a
/// full [`Arena`](super::Arena).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArenaFull;

impl Display for ArenaFull {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("Cannot reserve an key because the arena is full")
	}
}

impl Error for ArenaFull {}

/// Returned when trying to insert into an
/// [`Arena`](super::Arena) with an key that hasn't
/// been reserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyNotReserved;

impl Display for KeyNotReserved {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("Cannot insert with this key because it is not reserved")
	}
}

impl Error for KeyNotReserved {}
