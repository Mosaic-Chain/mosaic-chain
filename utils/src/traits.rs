pub trait Successor<T> {
	fn initial() -> T;
	fn successor(val: &T) -> T;
}

pub use crate::session_hook::SessionHook;
