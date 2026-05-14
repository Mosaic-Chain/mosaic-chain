//! PLACEHOLDER weights for `pallet_asset_conversion_tx_payment`

use core::marker::PhantomData;
use sdk::frame_support::{
	traits::Get,
	weights::{constants::RocksDbWeight, Weight},
};

use sdk::*;

use pallet_asset_conversion_tx_payment::weights::WeightInfo;

/// Weights for `pallet_asset_conversion` using the recommended hardware.
pub struct Weights<T>(PhantomData<T>);
impl<T: sdk::frame_system::Config> WeightInfo for Weights<T> {
	fn charge_asset_tx_payment_zero() -> Weight {
		Weight::MAX
	}

	fn charge_asset_tx_payment_native() -> Weight {
		Weight::MAX
	}

	fn charge_asset_tx_payment_asset() -> Weight {
		Weight::MAX
	}
}
