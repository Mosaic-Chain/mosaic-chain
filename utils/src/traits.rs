pub trait Successor<T> {
	fn initial() -> T;
	fn successor(val: &T) -> T;
}
