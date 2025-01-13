use sdk::{frame_support, pallet_parameters, sp_runtime};

use codec::{Decode, Encode, MaxEncodedLen};
use core::num::NonZeroU32;
use frame_support::dynamic_params::{dynamic_pallet_params, dynamic_params};
use scale_info::TypeInfo;
use sp_runtime::{Perbill, Permill};

use crate::BlockNumber;
use utils::prod_or_fast;

#[derive(Debug, Clone, Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq)]
pub struct PaymentRatio {
	pub validator: u32,
	pub treasury: u32,
	pub burn: u32,
}

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
	pub const YEARS: BlockNumber = 5259600;
}

#[dynamic_params(RuntimeParameters, pallet_parameters::Parameters::<crate::Runtime>)]
pub mod dynamic {
	use super::*;
	use currency::*;
	use time::*;

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
		pub static MinimumStakingAmount: Balance = MOSAIC;

		#[codec(index = 2)]
		pub static MinimumStakingPeriod: NonZeroU32 =
			unsafe { NonZeroU32::new_unchecked(prod_or_fast!(672, 2)) };

		#[codec(index = 3)]
		pub static MaximumStakePercentage: Perbill = Perbill::from_percent(5);

		/// Period after which a chilled validator is considered slacking
		#[codec(index = 4)]
		pub static SlackingPeriod: u32 = prod_or_fast!(72, 10);

		/// A percent under which a validator is disqualified
		#[codec(index = 5)]
		pub static NominalValueThreshold: Perbill = Perbill::from_percent(80);

		/// A percent of the distributed session reward that goes somewhere other than the stakers
		#[codec(index = 6)]
		pub static ContributionPercentage: Perbill = Perbill::from_percent(20);
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
		/// The time-out for motions.
		#[codec(index = 0)]
		pub static MotionDuration: BlockNumber = 28 * DAYS;
	}

	#[dynamic_pallet_params]
	#[codec(index = 7)]
	pub mod development_collective {
		/// The time-out for motions.
		#[codec(index = 0)]
		pub static MotionDuration: BlockNumber = 28 * DAYS;
	}

	#[dynamic_pallet_params]
	#[codec(index = 8)]
	pub mod financial_collective {
		/// The time-out for motions.
		#[codec(index = 0)]
		pub static MotionDuration: BlockNumber = 28 * DAYS;
	}

	#[dynamic_pallet_params]
	#[codec(index = 9)]
	pub mod community_collective {
		/// The time-out for motions.
		#[codec(index = 0)]
		pub static MotionDuration: BlockNumber = 28 * DAYS;
	}

	#[dynamic_pallet_params]
	#[codec(index = 10)]
	pub mod team_and_advisors_collective {
		/// The time-out for motions.
		#[codec(index = 0)]
		pub static MotionDuration: BlockNumber = 28 * DAYS;
	}

	#[dynamic_pallet_params]
	#[codec(index = 11)]
	pub mod security_collective {
		/// The time-out for motions.
		#[codec(index = 0)]
		pub static MotionDuration: BlockNumber = 28 * DAYS;
	}

	#[dynamic_pallet_params]
	#[codec(index = 12)]
	pub mod education_collective {
		/// The time-out for motions.
		#[codec(index = 0)]
		pub static MotionDuration: BlockNumber = 28 * DAYS;
	}

	#[dynamic_pallet_params]
	#[codec(index = 13)]
	pub mod treasury {
		/// Fraction of a proposal's value that should be bonded in order to place the proposal.
		/// An accepted proposal gets these back. A rejected proposal does not.
		#[codec(index = 0)]
		pub static ProposalBond: Permill = Permill::from_percent(1);

		/// Minimum amount of funds that should be placed in a deposit for making a proposal.
		#[codec(index = 1)]
		pub static ProposalBondMinimum: Balance = 10 * MOSAIC;

		/// Maximum amount of funds that should be placed in a deposit for making a proposal.
		#[codec(index = 2)]
		pub static ProposalBondMaximum: Option<Balance> = None;

		/// Period between successive spends.
		#[codec(index = 3)]
		pub static SpendPeriod: BlockNumber = prod_or_fast!(28 * DAYS, 10 * MINUTES);

		/// Percentage of spare funds (if any) that are burnt per spend period.
		#[codec(index = 4)]
		pub static Burn: Permill =
			prod_or_fast!(Permill::from_percent(1), Permill::from_perthousand(1));

		/// The period during which an approved treasury spend has to be claimed.
		#[codec(index = 5)]
		pub static PayoutPeriod: BlockNumber = prod_or_fast!(14 * DAYS, 5 * MINUTES);
	}

	#[dynamic_pallet_params]
	#[codec(index = 14)]
	pub mod development_fund {
		/// Fraction of a proposal's value that should be bonded in order to place the proposal.
		/// An accepted proposal gets these back. A rejected proposal does not.
		#[codec(index = 0)]
		pub static ProposalBond: Permill = Permill::from_percent(1);

		/// Minimum amount of funds that should be placed in a deposit for making a proposal.
		#[codec(index = 1)]
		pub static ProposalBondMinimum: Balance = 10 * MOSAIC;

		/// Maximum amount of funds that should be placed in a deposit for making a proposal.
		#[codec(index = 2)]
		pub static ProposalBondMaximum: Option<Balance> = None;

		/// Period between successive spends.
		#[codec(index = 3)]
		pub static SpendPeriod: BlockNumber = prod_or_fast!(28 * DAYS, 10 * MINUTES);

		/// Percentage of spare funds (if any) that are burnt per spend period.
		#[codec(index = 4)]
		pub static Burn: Permill =
			prod_or_fast!(Permill::from_percent(1), Permill::from_perthousand(1));

		/// The period during which an approved treasury spend has to be claimed.
		#[codec(index = 5)]
		pub static PayoutPeriod: BlockNumber = prod_or_fast!(14 * DAYS, 5 * MINUTES);
	}

	#[dynamic_pallet_params]
	#[codec(index = 15)]
	pub mod financial_fund {
		/// Fraction of a proposal's value that should be bonded in order to place the proposal.
		/// An accepted proposal gets these back. A rejected proposal does not.
		#[codec(index = 0)]
		pub static ProposalBond: Permill = Permill::from_percent(1);

		/// Minimum amount of funds that should be placed in a deposit for making a proposal.
		#[codec(index = 1)]
		pub static ProposalBondMinimum: Balance = 10 * MOSAIC;

		/// Maximum amount of funds that should be placed in a deposit for making a proposal.
		#[codec(index = 2)]
		pub static ProposalBondMaximum: Option<Balance> = None;

		/// Period between successive spends.
		#[codec(index = 3)]
		pub static SpendPeriod: BlockNumber = prod_or_fast!(28 * DAYS, 10 * MINUTES);

		/// Percentage of spare funds (if any) that are burnt per spend period.
		#[codec(index = 4)]
		pub static Burn: Permill =
			prod_or_fast!(Permill::from_percent(1), Permill::from_perthousand(1));

		/// The period during which an approved treasury spend has to be claimed.
		#[codec(index = 5)]
		pub static PayoutPeriod: BlockNumber = prod_or_fast!(14 * DAYS, 5 * MINUTES);
	}

	#[dynamic_pallet_params]
	#[codec(index = 16)]
	pub mod community_fund {
		/// Fraction of a proposal's value that should be bonded in order to place the proposal.
		/// An accepted proposal gets these back. A rejected proposal does not.
		#[codec(index = 0)]
		pub static ProposalBond: Permill = Permill::from_percent(1);

		/// Minimum amount of funds that should be placed in a deposit for making a proposal.
		#[codec(index = 1)]
		pub static ProposalBondMinimum: Balance = 10 * MOSAIC;

		/// Maximum amount of funds that should be placed in a deposit for making a proposal.
		#[codec(index = 2)]
		pub static ProposalBondMaximum: Option<Balance> = None;

		/// Period between successive spends.
		#[codec(index = 3)]
		pub static SpendPeriod: BlockNumber = prod_or_fast!(28 * DAYS, 10 * MINUTES);

		/// Percentage of spare funds (if any) that are burnt per spend period.
		#[codec(index = 4)]
		pub static Burn: Permill =
			prod_or_fast!(Permill::from_percent(1), Permill::from_perthousand(1));

		/// The period during which an approved treasury spend has to be claimed.
		#[codec(index = 5)]
		pub static PayoutPeriod: BlockNumber = prod_or_fast!(14 * DAYS, 5 * MINUTES);
	}

	#[dynamic_pallet_params]
	#[codec(index = 17)]
	pub mod team_and_advisors_fund {
		/// Fraction of a proposal's value that should be bonded in order to place the proposal.
		/// An accepted proposal gets these back. A rejected proposal does not.
		#[codec(index = 0)]
		pub static ProposalBond: Permill = Permill::from_percent(1);

		/// Minimum amount of funds that should be placed in a deposit for making a proposal.
		#[codec(index = 1)]
		pub static ProposalBondMinimum: Balance = 10 * MOSAIC;

		/// Maximum amount of funds that should be placed in a deposit for making a proposal.
		#[codec(index = 2)]
		pub static ProposalBondMaximum: Option<Balance> = None;

		/// Period between successive spends.
		#[codec(index = 3)]
		pub static SpendPeriod: BlockNumber = prod_or_fast!(28 * DAYS, 10 * MINUTES);

		/// Percentage of spare funds (if any) that are burnt per spend period.
		#[codec(index = 4)]
		pub static Burn: Permill =
			prod_or_fast!(Permill::from_percent(1), Permill::from_perthousand(1));

		/// The period during which an approved treasury spend has to be claimed.
		#[codec(index = 5)]
		pub static PayoutPeriod: BlockNumber = prod_or_fast!(14 * DAYS, 5 * MINUTES);
	}

	#[dynamic_pallet_params]
	#[codec(index = 18)]
	pub mod security_fund {
		/// Fraction of a proposal's value that should be bonded in order to place the proposal.
		/// An accepted proposal gets these back. A rejected proposal does not.
		#[codec(index = 0)]
		pub static ProposalBond: Permill = Permill::from_percent(1);

		/// Minimum amount of funds that should be placed in a deposit for making a proposal.
		#[codec(index = 1)]
		pub static ProposalBondMinimum: Balance = 10 * MOSAIC;

		/// Maximum amount of funds that should be placed in a deposit for making a proposal.
		#[codec(index = 2)]
		pub static ProposalBondMaximum: Option<Balance> = None;

		/// Period between successive spends.
		#[codec(index = 3)]
		pub static SpendPeriod: BlockNumber = prod_or_fast!(28 * DAYS, 10 * MINUTES);

		/// Percentage of spare funds (if any) that are burnt per spend period.
		#[codec(index = 4)]
		pub static Burn: Permill =
			prod_or_fast!(Permill::from_percent(1), Permill::from_perthousand(1));

		/// The period during which an approved treasury spend has to be claimed.
		#[codec(index = 5)]
		pub static PayoutPeriod: BlockNumber = prod_or_fast!(14 * DAYS, 5 * MINUTES);
	}

	#[dynamic_pallet_params]
	#[codec(index = 19)]
	pub mod education_fund {
		/// Fraction of a proposal's value that should be bonded in order to place the proposal.
		/// An accepted proposal gets these back. A rejected proposal does not.
		#[codec(index = 0)]
		pub static ProposalBond: Permill = Permill::from_percent(1);

		/// Minimum amount of funds that should be placed in a deposit for making a proposal.
		#[codec(index = 1)]
		pub static ProposalBondMinimum: Balance = 10 * MOSAIC;

		/// Maximum amount of funds that should be placed in a deposit for making a proposal.
		#[codec(index = 2)]
		pub static ProposalBondMaximum: Option<Balance> = None;

		/// Period between successive spends.
		#[codec(index = 3)]
		pub static SpendPeriod: BlockNumber = prod_or_fast!(28 * DAYS, 10 * MINUTES);

		/// Percentage of spare funds (if any) that are burnt per spend period.
		#[codec(index = 4)]
		pub static Burn: Permill =
			prod_or_fast!(Permill::from_percent(1), Permill::from_perthousand(1));

		/// The period during which an approved treasury spend has to be claimed.
		#[codec(index = 5)]
		pub static PayoutPeriod: BlockNumber = prod_or_fast!(14 * DAYS, 5 * MINUTES);
	}

	#[dynamic_pallet_params]
	#[codec(index = 20)]
	pub mod transaction_payment {
		#[codec(index = 0)]
		pub static FeePaymentRatio: PaymentRatio =
			PaymentRatio { validator: 50, treasury: 10, burn: 40 };
	}

	#[dynamic_pallet_params]
	#[codec(index = 21)]
	// With these settings an average session is between 55m and 1h05m
	// A validator is roughly selected 17 times a week given 2000 active validators.
	pub mod validator_subset_selection {
		/// The desired mean subset size to be selected.
		#[codec(index = 0)]
		pub static SubsetSize: u32 = prod_or_fast!(200, 3);

		/// The minimum length of a session.
		/// A session's maximum length is `MinSessionLength + SubsetSize - 1`
		#[codec(index = 1)]
		pub static MinSessionLength: BlockNumber = prod_or_fast!(45 * MINUTES, MINUTES);
	}

	#[dynamic_pallet_params]
	#[codec(index = 22)]
	/// Token generation constants.
	/// In production mode they are set to zero before the Token Generation Event.
	pub mod token_generation {
		/// The desired slash percentage.
		#[codec(index = 0)]
		pub static SlashFraction: Perbill =
			prod_or_fast!(Perbill::zero(), Perbill::from_perthousand(1));

		/// Multiplier use in reward calculation algorithm.
		#[codec(index = 1)]
		pub static TokenGenerationFactor: u64 = prod_or_fast!(0, 125 * (MOSAIC / 10) as u64);
	}
}

#[cfg(feature = "runtime-benchmarks")]
impl Default for RuntimeParameters {
	fn default() -> Self {
		RuntimeParameters::NftStaking(dynamic::nft_staking::Parameters::MinimumStakingAmount(
			dynamic::nft_staking::MinimumStakingAmount,
			Some(50 * currency::MOSAIC),
		))
	}
}
