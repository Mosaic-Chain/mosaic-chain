use sdk::{frame_support, frame_system, sp_runtime};

use frame_support::{
	parameter_types,
	weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight},
	PalletId,
};
use sp_runtime::Perbill;

use super::{
	currency::{CENTS, MOSAIC},
	time::{DAYS, SLOT_DURATION, YEARS},
};
use crate::{Balance, BlockNumber, RuntimeVersion, VERSION};

pub mod system {
	use super::*;

	const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

	parameter_types! {
		pub const BlockHashCount: BlockNumber = 2400;
		pub const Version: RuntimeVersion = VERSION;

		/// We allow for 2 seconds of compute with a 6 second average block time.
		pub BlockWeights: frame_system::limits::BlockWeights =
			frame_system::limits::BlockWeights::with_sensible_defaults(
				Weight::from_parts(2u64 * WEIGHT_REF_TIME_PER_SECOND, u64::MAX),
				NORMAL_DISPATCH_RATIO,
			);
		pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
			::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);

		pub const SS58Prefix: u16 = if cfg!(feature = "mainnet") { 14998 } else { 42 };
	}
}

pub mod assets {
	use super::*;

	parameter_types! {
		pub const StringLimit: u32 = 64;
		pub const RemoveItemsLimit: u32 = 32;
	}
}

pub mod aura {
	use super::*;

	parameter_types! {
		pub const MaxActiveAuthorities: u32 = 400;
		pub const AllowMultipleBlocksPerSlot: bool = true;
		pub const SlotDuration: u64 = SLOT_DURATION;
	}
}

pub mod timestamp {
	use super::*;

	parameter_types! {
		pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
	}
}

pub mod balances {
	use super::*;

	parameter_types! {
		pub const ExistentialDeposit: Balance = CENTS;
		pub const MaxLocks: u32 = 64;
		pub const MaxReserves: u32 = 64;
	}
}

pub mod transaction_payment {
	use super::*;
	use sdk::pallet_transaction_payment::Multiplier;
	use sp_runtime::{traits::Bounded, FixedPointNumber, Perquintill};

	parameter_types! {
		pub const OperationalFeeMultiplier: u8 = 5;
		/// The portion of the `NORMAL_DISPATCH_RATIO` that we adjust the fees with. Blocks filled less
		/// than this will decrease the weight and more will increase.
		pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
		/// The adjustment variable of the runtime. Higher values will cause `TargetBlockFullness` to
		/// change the fees more rapidly.
		pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(75, 1_000_000);
		/// Minimum amount of the multiplier. This value cannot be too low. A test case should ensure
		/// that combined with `AdjustmentVariable`, we can recover from the minimum.
		/// See `multiplier_can_grow_from_zero`.
		pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 10u128);
		/// The maximum amount of the multiplier.
		pub MaximumMultiplier: Multiplier = Bounded::max_value();

	}
}

pub mod nfts {
	use super::*;

	parameter_types! {
		pub const StringLimit: u32 = 256;
		pub const KeyLimit: u32 = 64;
		pub const ValueLimit: u32 = 256;
		pub const ApprovalsLimit: u32 = 20;
		pub const ItemAttributesApprovalsLimit: u32 = 30;
		pub const MaxTips: u32 = 10;
		pub const MaxDeadlineDuration: u32 = 7 * DAYS;
		pub const MaxAttributesPerCall: u32 = 10;
	}
}

pub mod nft_permission {
	use super::*;

	parameter_types! {
		pub const PalletId: super::PalletId = super::PalletId(*b"permissi");
	}
}

pub mod nft_staking {
	use super::*;

	parameter_types! {
		pub const MaximumContractsPerValidator: u32 = 1000;
		pub const MaximumBoundValidators: u32 = 4000;
	}
}

pub mod identity {
	use super::*;

	parameter_types! {
		pub const MaxSubAccounts: u32 = 16;
		pub const MaxAdditionalFields: u32 = 16;
		pub const MaxRegistrars: u32 = 32;
		pub const MaxSuffixLength: u32 = 7;
		pub const MaxUsernameLength: u32 = 32;
		pub const PendingUsernameExpiration: u32 = 7 * DAYS;
		pub const UsernameGracePeriod: u32 = 7 * DAYS;
		// Must only change in a runtime upgrade with proper migrations.
		pub const UsernameDeposit: Balance = MOSAIC; // currently = deposit(1, 0)
	}
}

pub mod scheduler {
	use super::*;

	parameter_types! {
		pub MaximumWeight: Weight = Perbill::from_percent(75) * system::BlockWeights::get().max_block;
		pub const MaxScheduledPerBlock: u32 = 16;
	}
}

pub mod preimage {
	use super::*;
	use crate::RuntimeHoldReason;

	parameter_types! {
		pub const HoldReason: RuntimeHoldReason = RuntimeHoldReason::Preimage(sdk::pallet_preimage::HoldReason::Preimage);
	}
}

pub mod proxy {
	use super::*;

	parameter_types! {
		/// Maximum number of proxies per account
		pub const MaxProxies: u32 = 32;
		/// Maximum number of pending announcements per account
		pub const MaxPending: u32 = 32;
	}
}

pub mod recovery {
	use super::*;

	parameter_types! {
		pub const MaxFriends: u16 = 9;
	}
}

pub mod im_online {
	use super::*;
	use sp_runtime::transaction_validity::TransactionPriority;

	parameter_types! {
		pub const UnsignedPriority: TransactionPriority = TransactionPriority::MAX;
	}
}

pub mod nft_delegation {
	use super::*;

	parameter_types! {
		pub const PalletId: super::PalletId = super::PalletId(*b"delegati");
		pub const MaxExpirationsPerSession: u32 = 16;
	}
}

pub mod airdrop {
	use super::*;
	use sp_runtime::transaction_validity::TransactionPriority;

	parameter_types! {
		pub const BaseTransactionPriority: u64 = TransactionPriority::MAX / 2;
		pub const MaxAirdropsInPool: u64 = 12;
	}

	pub const MAX_DELEGATOR_NFTS: u32 = 5;
}

pub mod hold_vesting {
	use super::*;

	parameter_types! {
		pub const MinVestedTransfer: Balance = 50 * MOSAIC;
	}

	pub const MAX_VESTING_SCHEDULES: u32 = 8;
}

pub mod vesting_to_freeze {
	use super::*;
	use frame_support::traits::VariantCount;

	parameter_types! {
		pub const MaxFrozenSchedules: u32 = 8;
		pub MaxFreezes: u32 = balances::MaxLocks::get() + crate::RuntimeFreezeReason::VARIANT_COUNT;
		pub const MaxVestingSchedules: u32 = hold_vesting::MAX_VESTING_SCHEDULES;
	}
}

pub mod staking_incentive {
	use super::*;
	use sp_runtime::FixedU128;

	parameter_types! {
		pub const PerBlockMultiplier: FixedU128 = utils::prod_or_fast!(
			// 2x growth over a year: exp(ln(2)/5259600)
			FixedU128::from_rational(1_000_000_131_787_061_037, 1_000_000_000_000_000_000),
			// At max supply an overflow occurs in 269 blocks: (128-91) * ln(2)/ln(1.1)
			FixedU128::from_rational(11, 10)
		);
		pub const PalletId: super::PalletId = super::PalletId(*b"mstkince");
		pub const ClaimVestingScheduleLength: u32 = YEARS;
		pub const MaxPayouts: u32 = 16;
	}
}

// Configuration for every instance
pub mod membership {
	use super::*;

	parameter_types! {
		pub const MaxMembers: u32 = 100;
	}
}

// Configuration for every instance
pub mod collective {
	use super::*;

	parameter_types! {
		pub const MaxMembers: u32 = membership::MaxMembers::get();
		pub const MaxProposals: u32 = 10;
		pub MaxProposalWeight: Weight = Perbill::from_percent(75) * system::BlockWeights::get().max_block;
	}
}

// Configuration for every instance
pub mod treasury {
	use super::*;

	parameter_types! {
		pub const MaxApprovals: u32 = 100;
		pub const MaxBalance: Balance = Balance::MAX;
	}
}
