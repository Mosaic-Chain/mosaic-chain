use frame_support::{
	pallet_prelude::{Decode, Encode, TypeInfo},
	sp_runtime::RuntimeDebug,
};

//TODO: consider using `Compact`
//TODO: consider making committed not optional
/// Adds a "staging" overlay to a value.
/// Useful when managing the transition between a last "stable" or "active" state
/// and a potential new state applied in the next period, providing incremental updates.
#[derive(Encode, Decode, RuntimeDebug, TypeInfo, Copy, Clone)]
pub struct Staging<T> {
	staged: Option<T>,
	committed: Option<T>,
}

impl<T> Default for Staging<T> {
	fn default() -> Self {
		Self { staged: None, committed: None }
	}
}

impl<T> Staging<T> {
	/// Creates a new `Staging` instance with a committed value and no staged value.
	pub fn new(value: T) -> Self {
		Self { staged: None, committed: Some(value) }
	}

	/// Creates a new `Staging` instance with a staged value and no committed value.
	pub fn new_staged(value: T) -> Self {
		Self { staged: Some(value), committed: None }
	}

	/// Clears both staged and commited values
	pub fn purge(&mut self) {
		self.committed = None;
		self.staged = None;
	}

	/// Returns the committed value, if present.
	#[must_use]
	pub fn committed(&self) -> Option<&T> {
		self.committed.as_ref()
	}

	/// Returns the current value, preferring the staged value if present.
	#[must_use]
	pub fn current(&self) -> Option<&T> {
		self.staged.as_ref().or(self.committed.as_ref())
	}

	/// Stages a new value, overwriting the previous staged value.
	pub fn stage(&mut self, value: T) {
		self.staged = Some(value);
	}

	/// Commits the staged value, updating the committed value.
	pub fn commit(&mut self) {
		self.committed = self.staged.take().or(self.committed.take());
	}

	/// Checks if a committed value exists.
	pub fn exists_committed(&self) -> bool {
		self.committed.is_some()
	}

	/// Checks if either a staged or committed value exists.
	pub fn exists(&self) -> bool {
		self.staged.is_some() || self.committed.is_some()
	}
}
