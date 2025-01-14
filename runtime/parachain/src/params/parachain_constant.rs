use sdk::frame_support::{parameter_types, weights::Weight};

use sdk::sp_runtime::Perbill;

use super::{constant::system, currency::message_fee};

pub mod parachain_system {
	use super::*;
	use sdk::cumulus_primitives_core::AggregateMessageOrigin;

	/// Maximum number of blocks simultaneously accepted by the Runtime, not yet included
	/// into the relay chain.
	pub const UNINCLUDED_SEGMENT_CAPACITY: u32 = 3;

	/// How many parachain blocks are processed by the relay chain per parent. Limits the
	/// number of blocks authored per slot.
	pub const BLOCK_PROCESSING_VELOCITY: u32 = 1;

	/// Relay chain slot duration, in milliseconds.
	pub const RELAY_CHAIN_SLOT_DURATION_MILLIS: u32 = 6000;

	parameter_types! {
		pub ReservedXcmpWeight: Weight = system::BlockWeights::get().max_block.saturating_div(4);
		pub ReservedDmpWeight: Weight = system::BlockWeights::get().max_block.saturating_div(4);
		pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
	}
}

pub mod message_queue {
	use super::*;

	parameter_types! {
		pub ServiceWeight: Weight = Perbill::from_percent(35) * system::BlockWeights::get().max_block;
		pub const HeapSize: u32 = 64 * 1024;
		pub const MaxStale: u32 = 8;
	}
}
pub mod xcmp_queue {
	use super::*;
	use crate::xcm_config;
	use sdk::cumulus_primitives_core::AssetId;

	parameter_types! {
		/// The asset ID for the asset that we use to pay for message delivery fees.
		pub FeeAssetId: AssetId = AssetId(xcm_config::TokenLocation::get());
		/// The base fee for the message delivery fees.
		pub const BaseDeliveryFee: u128 = message_fee(1, 0);
		/// The fee for the message delivery per byte added to the base fee.
		pub const MessageByteFee: u128 = message_fee(0, 1);
		pub const MaxInboundSuspended: u32 = 1000;
		pub const MaxActiveOutboundChannels: u32 = 128;
		pub const MaxPageSize: u32 = 1 << 16;
	}
}
