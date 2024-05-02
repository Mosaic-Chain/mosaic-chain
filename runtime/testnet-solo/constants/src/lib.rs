#![cfg_attr(not(feature = "std"), no_std)]

/// Money related constants
pub mod currency {
	/// Balance of an account.
	pub type Balance = u128;

	pub const MOSAIC: Balance = 10u128.pow(18);
	pub const DOLLAR: Balance = MOSAIC;
	pub const CENTS: Balance = MOSAIC / 100;
	pub const MILLICENTS: Balance = MOSAIC / 1_000;
	pub const GRAND: Balance = MOSAIC * 1000;

	// TODO: rewrite values
	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 20 * MOSAIC + (bytes as Balance) * 10 * CENTS
	}
}
