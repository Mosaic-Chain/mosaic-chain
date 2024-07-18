use crate::BlockNumber;
use core::num::NonZeroU32;
use frame_support::dynamic_params::{dynamic_pallet_params, dynamic_params};
use sp_runtime::Perbill;

pub mod currency {
	pub type Balance = u128;

	pub const MOSAIC: Balance = 10u128.pow(18);
	pub const CENTS: Balance = MOSAIC / 100;

	pub const fn deposit(items: u32, bytes: u32) -> Balance {
		items as Balance * 20 * MOSAIC + (bytes as Balance) * 10 * CENTS
	}
}

pub mod time {
	use super::BlockNumber;

	// NOTE: Currently it is not possible to change the slot duration after the chain has started.
	//       Attempting to do so will brick block production.
	pub const SLOT_DURATION: u64 = 6000;

	pub const MINUTES: BlockNumber = 60_000 / (SLOT_DURATION as BlockNumber);
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;
}

#[dynamic_params(RuntimeParameters, pallet_parameters::Parameters::<crate::Runtime>)]
pub mod dynamic {
	use super::*;
	use currency::*;
	use time::MINUTES;

	#[dynamic_pallet_params]
	#[codec(index = 0)]
	pub mod assets {
		/// The basic amount of funds that must be reserved for an asset.
		#[codec(index = 0)]
		pub static AssetDeposit: Balance = deposit(1, 0);

		/// The amount of funds that must be reserved for a non-provider asset account to be maintained.
		#[codec(index = 1)]
		pub static AssetAccountDeposit: Balance = deposit(1, 16);

		/// The basic amount of funds that must be reserved when adding metadata to your asset.
		#[codec(index = 2)]
		pub static MetadataDepositBase: Balance = deposit(1, 68);

		/// The additional funds that must be reserved for the number of bytes you store in your metadata.
		#[codec(index = 3)]
		pub static MetadataDepositPerByte: Balance = deposit(0, 1);

		/// The amount of funds that must be reserved when creating a new approval.
		#[codec(index = 4)]
		pub static ApprovalDeposit: Balance = CENTS / 100;
	}

	#[dynamic_pallet_params]
	#[codec(index = 1)]
	pub mod nft_staking {
		#[codec(index = 0)]
		pub static MinimumCommission: Perbill = Perbill::from_percent(1);

		#[codec(index = 1)]
		pub static MinimumStakingAmount: Balance = 10;

		#[codec(index = 2)]
		pub static MinimumStakingPeriod: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(200) }; // approx. 1 week

		#[codec(index = 3)]
		pub static MaximumStakePercentage: Perbill = Perbill::from_percent(1);

		/// Period after which a chilled validator is considered slacking
		#[codec(index = 4)]
		pub static SlackingPeriod: u32 = 10;

		/// A percent under which a validator is disqualified
		#[codec(index = 5)]
		pub static NominalValueThreshold: Perbill = Perbill::from_percent(80);
	}

	#[dynamic_pallet_params]
	#[codec(index = 2)]
	pub mod identity {
		/// The amount held on deposit per encoded byte for a registered identity.
		#[codec(index = 0)]
		pub static ByteDeposit: Balance = deposit(0, 1);

		/// The amount held on deposit for a registered identity.
		#[codec(index = 1)]
		pub static BasicDeposit: Balance = deposit(1, 258);

		/// The amount held on deposit for a registered subaccount.
		#[codec(index = 2)]
		pub static SubAccountDeposit: Balance = deposit(1, 53);
	}

	#[dynamic_pallet_params]
	#[codec(index = 3)]
	pub mod preimage {
		#[codec(index = 0)]
		pub static ByteDeposit: Balance = deposit(2, 64);

		#[codec(index = 1)]
		pub static BaseDeposit: Balance = deposit(0, 1);
	}

	#[dynamic_pallet_params]
	#[codec(index = 4)]
	pub mod proxy {
		/// The base amount of currency needed to reserve for creating a proxy.
		#[codec(index = 0)]
		pub static DepositBase: Balance = deposit(1, 8);

		/// The amount of currency needed per proxy added.
		#[codec(index = 1)]
		pub static DepositFactor: Balance = deposit(0, 33);

		/// The base amount of currency needed to reserve for creating an announcement.
		#[codec(index = 2)]
		pub static AnnouncementDepositBase: Balance = deposit(1, 16);

		/// The amount of currency needed per announcement made.
		#[codec(index = 3)]
		pub static AnnouncementDepositFactor: Balance = deposit(0, 68);
	}

	#[dynamic_pallet_params]
	#[codec(index = 5)]
	pub mod recovery {
		/// The base amount of currency needed to reserve for creating a recovery configuration.
		#[codec(index = 0)]
		pub static ConfigDepositBase: Balance = 500 * CENTS;

		/// The amount of currency needed per additional user when creating a recovery
		/// configuration.
		#[codec(index = 1)]
		pub static FriendDepositFactor: Balance = 50 * CENTS;

		/// The base amount of currency needed to reserve for starting a recovery.
		#[codec(index = 2)]
		pub static RecoveryDeposit: Balance = 500 * CENTS;
	}

	#[dynamic_pallet_params]
	#[codec(index = 6)]
	pub mod council {
		/// The time-out for council motions.
		#[codec(index = 0)]
		pub static MotionDuration: BlockNumber = MINUTES;
	}
}
