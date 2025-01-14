use sdk::sp_core::Get;

pub type Balance = u128;

pub const MOSAIC: Balance = 10u128.pow(18);
pub const CENTS: Balance = MOSAIC / 100;

/// Calculate deposit based on no. of items and bytes
///
/// This function is dynamic since it reads storage values defined by `dynamic_params`
pub fn deposit(items: u32, bytes: u32) -> Balance {
	items as Balance * super::dynamic::tokenomics::BaseDeposit::get()
		+ (bytes as Balance) * super::dynamic::tokenomics::PerByteDeposit::get()
}

pub const fn message_fee(base: u32, bytes: u32) -> Balance {
	base as Balance * MOSAIC + (bytes as Balance) * CENTS / 10
}
