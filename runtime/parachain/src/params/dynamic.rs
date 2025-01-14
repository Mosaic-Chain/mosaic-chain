use sdk::{frame_support, pallet_parameters, sp_runtime};

use codec::{Decode, Encode, MaxEncodedLen};
use core::num::NonZeroU32;
use frame_support::dynamic_params::{dynamic_pallet_params, dynamic_params};
use scale_info::TypeInfo;
use sp_runtime::{Perbill, Permill};

use crate::BlockNumber;
use utils::prod_or_fast;

use super::{currency, time};

pub use imp::*;

#[derive(Debug, Clone, Encode, Decode, MaxEncodedLen, TypeInfo, PartialEq, Eq)]
pub struct PaymentRatio {
	pub validator: u32,
	pub treasury: u32,
	pub burn: u32,
}

/// Dynamic constants for `pallet_assets`
///
/// These params are dynamic since the `deposit()`
/// function reads storage values defined by `dynamic_params`
pub mod assets {
	use super::*;
	use currency::{deposit, Balance};
	use frame_support::parameter_types;

	parameter_types! {
		pub AssetDeposit: Balance = deposit(1, 0);
		pub AssetAccountDeposit: Balance = deposit(1, 16);
		pub MetadataDepositBase: Balance = deposit(1, 68);
		pub MetadataDepositPerByte: Balance = deposit(0, 1);
		pub ApprovalDeposit: Balance = deposit(1, 10) / 10;
	}
}

/// Dynamic constants for `pallet_nfts`
///
/// These params are dynamic since the `deposit()`
/// function reads storage values defined by `dynamic_params`
pub mod nfts {
	use super::*;
	use currency::{deposit, Balance};
	use frame_support::parameter_types;

	parameter_types! {
		pub CollectionDeposit: Balance = deposit(1, 130) * 10;
		pub ItemDeposit: Balance = deposit(1, 164);
		pub MetadataDepositBase: Balance = deposit(1, 129) / 10;
		pub AttributeDepositBase: Balance = deposit(1, 0) / 10;
		pub DepositPerByte: Balance = deposit(0, 1);
	}
}

/// Dynamic constants for `pallet_identity`
///
/// These params are dynamic since the `deposit()`
/// function reads storage values defined by `dynamic_params`
pub mod identity {
	use super::*;
	use currency::{deposit, Balance};
	use frame_support::parameter_types;

	parameter_types! {
		/// The amount held on deposit per encoded byte for a registered identity.
		pub ByteDeposit: Balance = deposit(0, 1);

		/// The amount held on deposit for a registered identity.
		pub BasicDeposit: Balance = deposit(1, 258);

		/// The amount held on deposit for a registered subaccount.
		pub SubAccountDeposit: Balance = deposit(1, 53);
	}
}

/// Dynamic constants for `pallet_preimage`
///
/// These params are dynamic since the `deposit()`
/// function reads storage values defined by `dynamic_params`
pub mod preimage {
	use super::*;
	use currency::{deposit, Balance};
	use frame_support::parameter_types;

	parameter_types! {
		pub BaseDeposit: Balance = deposit(2, 64);
		pub ByteDeposit: Balance = deposit(0, 1);
	}
}

/// Dynamic constants for `pallet_proxy`
///
/// These params are dynamic since the `deposit()`
/// function reads storage values defined by `dynamic_params`
pub mod proxy {
	use super::*;
	use currency::{deposit, Balance};
	use frame_support::parameter_types;

	parameter_types! {
		/// The base amount of currency needed to reserve for creating a proxy.
		pub DepositBase: Balance = deposit(1, 8);

		/// The amount of currency needed per proxy added.
		pub DepositFactor: Balance = deposit(0, 33);

		/// The base amount of currency needed to reserve for creating an announcement.
		pub AnnouncementDepositBase: Balance = deposit(1, 16);

		/// The amount of currency needed per announcement made.
		pub AnnouncementDepositFactor: Balance = deposit(0, 68);
	}
}

/// Dynamic constants for `pallet_recovery`
///
/// These params are dynamic since the `deposit()`
/// function reads storage values defined by `dynamic_params`
pub mod recovery {
	use super::*;
	use currency::{deposit, Balance};
	use frame_support::parameter_types;

	parameter_types! {
		/// The base amount of currency needed to reserve for creating a recovery configuration.
		pub ConfigDepositBase: Balance = deposit(1, 0);

		/// The amount of currency needed per additional user when creating a recovery
		/// configuration.
		pub FriendDepositFactor: Balance = deposit(0, 32);

		/// The base amount of currency needed to reserve for starting a recovery.
		pub RecoveryDeposit: Balance = deposit(1, 0);
	}
}

#[dynamic_params(RuntimeParameters, pallet_parameters::Parameters::<crate::Runtime>)]
mod imp {
	use super::*;

	use super::currency::*;
	use time::*;

	#[dynamic_pallet_params]
	#[codec(index = 0)]
	pub mod tokenomics {
		/// The desired slash percentage.
		/// In production mode is's set to zero before the Token Generation Event.
		#[codec(index = 0)]
		pub static SlashFraction: Perbill =
			prod_or_fast!(Perbill::zero(), Perbill::from_perthousand(1));

		/// Multiplier use in reward calculation algorithm.
		/// In production mode is's set to zero before the Token Generation Event.
		#[codec(index = 1)]
		pub static TokenGenerationFactor: u64 = prod_or_fast!(0, 125 * (MOSAIC / 10) as u64);

		#[codec(index = 2)]
		pub static BaseDeposit: Balance = MOSAIC;

		#[codec(index = 3)]
		pub static PerByteDeposit: Balance = CENTS / 10;
	}

	#[dynamic_pallet_params]
	#[codec(index = 1)]
	pub mod nft_staking {
		#[codec(index = 0)]
		pub static MinimumCommission: Perbill = Perbill::from_percent(1);

		#[codec(index = 1)]
		pub static MinimumStakingAmount: Balance = 50 * MOSAIC;

		#[codec(index = 2)]
		pub static MinimumStakingPeriod: NonZeroU32 =
			unsafe { NonZeroU32::new_unchecked(prod_or_fast!(672, 2)) };

		#[codec(index = 3)]
		pub static MaximumStakePercentage: Perbill = Perbill::from_percent(5);

		/// Period after which a chilled validator is considered slacking
		#[codec(index = 4)]
		pub static SlackingPeriod: u32 = prod_or_fast!(72, 1);

		/// A percent under which a validator is disqualified
		#[codec(index = 5)]
		pub static NominalValueThreshold: Perbill = Perbill::from_percent(80);

		/// A percent of the distributed session reward that goes somewhere other than the stakers
		#[codec(index = 6)]
		pub static ContributionPercentage: Perbill = Perbill::from_percent(20);
	}

	#[dynamic_pallet_params]
	#[codec(index = 2)]
	pub mod council {
		/// The time-out for motions.
		#[codec(index = 0)]
		pub static MotionDuration: BlockNumber = 28 * DAYS;
	}

	#[dynamic_pallet_params]
	#[codec(index = 3)]
	pub mod development_collective {
		/// The time-out for motions.
		#[codec(index = 0)]
		pub static MotionDuration: BlockNumber = 28 * DAYS;
	}

	#[dynamic_pallet_params]
	#[codec(index = 4)]
	pub mod financial_collective {
		/// The time-out for motions.
		#[codec(index = 0)]
		pub static MotionDuration: BlockNumber = 28 * DAYS;
	}

	#[dynamic_pallet_params]
	#[codec(index = 5)]
	pub mod community_collective {
		/// The time-out for motions.
		#[codec(index = 0)]
		pub static MotionDuration: BlockNumber = 28 * DAYS;
	}

	#[dynamic_pallet_params]
	#[codec(index = 6)]
	pub mod team_and_advisors_collective {
		/// The time-out for motions.
		#[codec(index = 0)]
		pub static MotionDuration: BlockNumber = 28 * DAYS;
	}

	#[dynamic_pallet_params]
	#[codec(index = 7)]
	pub mod security_collective {
		/// The time-out for motions.
		#[codec(index = 0)]
		pub static MotionDuration: BlockNumber = 28 * DAYS;
	}

	#[dynamic_pallet_params]
	#[codec(index = 8)]
	pub mod education_collective {
		/// The time-out for motions.
		#[codec(index = 0)]
		pub static MotionDuration: BlockNumber = 28 * DAYS;
	}

	#[dynamic_pallet_params]
	#[codec(index = 9)]
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
	#[codec(index = 10)]
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
	#[codec(index = 11)]
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
	#[codec(index = 12)]
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
	#[codec(index = 13)]
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
	#[codec(index = 14)]
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
	#[codec(index = 15)]
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
	#[codec(index = 16)]
	pub mod transaction_payment {
		#[codec(index = 0)]
		pub static FeePaymentRatio: PaymentRatio =
			PaymentRatio { validator: 50, treasury: 10, burn: 40 };
	}

	#[dynamic_pallet_params]
	#[codec(index = 17)]
	// With these settings an average session is between 55m and 1h05m
	// A given validator is roughly selected 17 times a week given 2000 active validators.
	pub mod validator_subset_selection {
		/// The desired mean subset size to be selected.
		#[codec(index = 0)]
		pub static SubsetSize: u32 = prod_or_fast!(200, 3);

		/// The minimum length of a session.
		/// A session's maximum length is `MinSessionLength + SubsetSize - 1`
		#[codec(index = 1)]
		pub static MinSessionLength: BlockNumber = prod_or_fast!(45 * MINUTES, MINUTES);
	}
}

#[cfg(feature = "runtime-benchmarks")]
impl Default for RuntimeParameters {
	fn default() -> Self {
		RuntimeParameters::NftStaking(imp::nft_staking::Parameters::MinimumStakingAmount(
			imp::nft_staking::MinimumStakingAmount,
			Some(50 * currency::MOSAIC),
		))
	}
}
