// construct_runtime procmacro creates a __hidden_use_of_unchecked_extrinsic type name that rustc frowns upon:
#![allow(non_camel_case_types)]
#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use core::num::NonZeroU32;

use sp_std::prelude::*;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	traits::{
		fungible::HoldConsideration, AsEnsureOriginWithArg, EitherOfDiverse, EqualPrivilegeOnly,
		InstanceFilter, LinearStoragePrice,
	},
	PalletId,
};
use frame_system::{pallet_prelude::BlockNumberFor, EnsureRoot};
use pallet_grandpa::AuthorityId as GrandpaId;
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::KeyTypeId, ConstBool, OpaqueMetadata};
use sp_runtime::{
	create_runtime_str,
	generic::{self, Era},
	impl_opaque_keys,
	traits::{
		AccountIdLookup, BlakeTwo256, Block as BlockT, Convert, ConvertInto, IdentifyAccount,
		NumberFor, One, OpaqueKeys, Verify,
	},
	transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, ExtrinsicInclusionMode, MultiSignature, SaturatedConversion,
};
use sp_staking::offence::{Offence, ReportOffence};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
	construct_runtime, parameter_types,
	traits::{
		ConstU128, ConstU32, ConstU64, ConstU8, KeyOwnerProofSystem, Randomness, StorageInfo,
	},
	weights::{
		constants::{
			BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
		},
		IdentityFee, Weight,
	},
	StorageValue,
};
pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::{ConstFeeMultiplier, CurrencyAdapter, Multiplier};
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill};

pub use pallet_insecure_randomness_collective_flip;
pub use pallet_nft_permission;
pub use pallet_validator_subset_selection;

use pallet_im_online::sr25519::AuthorityId as ImOnlineId;

pub use mosaic_testnet_solo_constants::currency::{deposit, Balance, CENTS, MOSAIC};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

pub type ValidatorId = AccountId;

///Number of previous transactions associated with an account
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
	use super::*;

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub aura: Aura,
			pub grandpa: Grandpa,
			pub im_online: ImOnline,
		}
	}
}

// To learn more about runtime versioning, see:
// https://docs.substrate.io/main-docs/build/upgrade#runtime-versioning
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("mosaic-testnet-solo"),
	impl_name: create_runtime_str!("mosaic-testnet-solo"),
	authoring_version: 1,
	// The version of the runtime specification. A full node will not attempt to use its native
	//   runtime in substitute for the on-chain Wasm runtime unless all of `spec_name`,
	//   `spec_version`, and `authoring_version` are the same between Wasm and native.
	// This value is set to 100 to notify Polkadot-JS App (https://polkadot.js.org/apps) to use
	//   the compatible custom types.
	spec_version: 100,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 1,
};

/// This determines the average expected block time that we are targeting.
/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_aura` to implement `fn slot_duration()`.
///
/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 6000;

// NOTE: Currently it is not possible to change the slot duration after the chain has started.
//       Attempting to do so will brick block production.
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

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
	pub const SS58Prefix: u8 = 42;
}

// Configure FRAME pallets to include in runtime.
impl frame_system::Config for Runtime {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = frame_support::traits::Everything;

	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = BlockWeights;

	/// The maximum length of a block (in bytes).
	type BlockLength = BlockLength;

	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;

	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;

	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;

	/// The block type for the runtime
	type Block = Block;

	/// Number of previous transactions associated with an account
	type Nonce = Nonce;

	/// The type for hashing blocks and tries.
	type Hash = Hash;

	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;

	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;

	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;

	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;

	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;

	/// Version of the runtime.
	type Version = Version;

	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = PalletInfo;

	/// What to do if a new account is created.
	type OnNewAccount = ();

	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();

	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;

	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();

	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;

	/// The set code logic, just the default since we're not a parachain.
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;

	type RuntimeTask = ();

	type SingleBlockMigrations = ();

	type MultiBlockMigrator = ();

	type PreInherents = ();

	type PostInherents = ();
	type PostTransactions = ();
}

parameter_types! {
	// TODO: Review the amount and adjust it to the desired one if necessary
	pub const AssetDeposit: Balance = deposit(1, 0);
	pub const AssetAccountDeposit: Balance = deposit(1, 16);
	pub const MetadataDepositBase: Balance = deposit(1, 68);
	pub const MetadataDepositPerByte: Balance = deposit(0, 1);
	pub const ApprovalDeposit: Balance = CENTS/100;
}

impl pallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	// Max number of items to destroy per destroy_accounts and destroy_approvals call.
	type RemoveItemsLimit = ConstU32<32>;
	type AssetId = u64;
	type AssetIdParameter = u64;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId>>;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	// See macros above for the amounts
	// The basic amount of funds that must be reserved for an asset.
	type AssetDeposit = AssetDeposit;
	// The amount of funds that must be reserved for a non-provider asset account to be maintained.
	type AssetAccountDeposit = AssetAccountDeposit;
	// The basic amount of funds that must be reserved when adding metadata to your asset.
	type MetadataDepositBase = MetadataDepositBase;
	// The additional funds that must be reserved for the number of bytes you store in your metadata.
	type MetadataDepositPerByte = MetadataDepositPerByte;
	// The amount of funds that must be reserved when creating a new approval.
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = ConstU32<256>;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();
	type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<32>;
	type AllowMultipleBlocksPerSlot = ConstBool<false>;
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type MaxAuthorities = ConstU32<32>;
	type MaxSetIdSessionEntries = ConstU64<0>;
	type KeyOwnerProof = sp_core::Void;
	type EquivocationReportSystem = ();
	// We have no such things as nominators, grandpa only uses this for weight calulation of offence reporting extrinsics.
	// Since we use the default implementation of EquivocationReportSystem (noop), we can safely set this to 0.
	type MaxNominators = ConstU32<0>;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	type WeightInfo = ();
}

/// Existential deposit.
pub const EXISTENTIAL_DEPOSIT: u128 = 500;

impl pallet_balances::Config for Runtime {
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];

	/// The type for recording an account's balance.
	type Balance = Balance;

	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Self>;
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeHoldReason;
}

parameter_types! {
	pub FeeMultiplier: Multiplier = Multiplier::one();
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = IdentityFee<Balance>;
	type LengthToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
}

impl pallet_nfts::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId>>;
	type Locker = ();
	type CollectionDeposit = ();
	type ItemDeposit = ();
	type MetadataDepositBase = ();
	type AttributeDepositBase = ();
	type DepositPerByte = ();
	type StringLimit = ConstU32<256>;
	type KeyLimit = ConstU32<256>;
	type ValueLimit = ConstU32<256>;
	type ApprovalsLimit = ();
	type ItemAttributesApprovalsLimit = ();
	type MaxTips = ();
	type MaxDeadlineDuration = ();
	type MaxAttributesPerCall = ();
	type Features = ();
	type OffchainSignature = Signature;
	type OffchainPublic = <Signature as Verify>::Signer;
	type WeightInfo = pallet_nfts::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const NftPermissionPalletId: PalletId = PalletId(*b"nft_perm");
}

impl pallet_nft_permission::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type WeightInfo = pallet_nft_permission::SubstrateWeight<Runtime>;
	type PalletId = NftPermissionPalletId;
	type PrivilegedOrigin = frame_system::EnsureRoot<AccountId>;
	type Permission = pallet_nft_staking::PermissionType;
}

parameter_types! {
	pub const MinimumCommission: Perbill = Perbill::from_percent(1);
	pub const StakingPalletId: PalletId = PalletId(*b"mstaking");
	pub const MinimumStakingAmount: Balance = 10;
	pub const MinimumStakingPeriod: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(200) }; // approx. 1 week
	pub const NominalValueThreshold: Perbill = Perbill::from_percent(80);
	// TODO: revise this value
	pub const MaximumStakePercentage: Perbill = Perbill::from_percent(1);
	pub const MaximumContractsPerValidator: u32 = 1000;
}

pub struct IdTupleToValidatorId;

impl Convert<IdTuple, AccountId> for IdTupleToValidatorId {
	fn convert(id_tuple: IdTuple) -> AccountId {
		id_tuple.0
	}
}

impl pallet_nft_staking::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type NftDelegationHandler = NftDelegation;
	type NftStakingHandler = NftPermission;
	type Balance = Balance;
	type ItemId = <Self as pallet_nfts::Config>::ItemId;
	type PalletId = StakingPalletId;

	type SlackingPeriod = ConstU32<10>; // approx. 8hrs
	type NominalValueThreshold = NominalValueThreshold;
	type MinimumStakingPeriod = MinimumStakingPeriod;
	type MinimumCommissionRate = MinimumCommission;
	type MinimumStakingAmount = MinimumStakingAmount;
	type MaximumStakePercentage = MaximumStakePercentage;
	type MaximumContractsPerValidator = MaximumContractsPerValidator;

	type SessionReward = ConstU128<1000>; // TODO: substitue with mechanism based on "eras/sections". (ever-decreasing rewards)
	type OnReward = ();

	type OffenderToValidatorId = IdTupleToValidatorId;
}

parameter_types! {
	// TODO: review these
	pub const ByteDeposit: Balance = deposit(0, 1);
	pub const BasicDeposit: Balance = deposit(1, 258);
	pub const SubAccountDeposit: Balance = deposit(1, 53);
	pub const MaxSubAccounts: u32 = 100;
	pub const MaxAdditionalFields: u32 = 100;
	pub const MaxRegistrars: u32 = 20;
	pub const MaxSuffixLength: u32 = 20;
	pub const MaxUsernameLength: u32 = 20;
}

impl pallet_identity::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BasicDeposit = BasicDeposit;
	type SubAccountDeposit = SubAccountDeposit;
	type MaxSubAccounts = MaxSubAccounts;
	type MaxRegistrars = MaxRegistrars;
	type Slashed = ();
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type RegistrarOrigin = frame_system::EnsureRoot<AccountId>;
	type WeightInfo = pallet_identity::weights::SubstrateWeight<Runtime>;
	type ByteDeposit = ByteDeposit;
	type IdentityInformation = pallet_identity::legacy::IdentityInfo<MaxAdditionalFields>;
	type OffchainSignature = Signature;
	type SigningPublicKey = <Signature as Verify>::Signer;
	type UsernameAuthorityOrigin = frame_system::EnsureRoot<AccountId>;
	type PendingUsernameExpiration = ConstU32<{ 7 * DAYS }>;
	type MaxSuffixLength = MaxSuffixLength;
	type MaxUsernameLength = MaxUsernameLength;
}

parameter_types! {
	pub MaximumWeight: Weight = Perbill::from_percent(75) * BlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
}

impl pallet_scheduler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumWeight;
	type ScheduleOrigin =
		EitherOfDiverse<frame_system::EnsureRoot<AccountId>, frame_system::EnsureSigned<AccountId>>;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
	type Preimages = Preimage;
}

parameter_types! {
	pub const PreimageBaseDeposit: Balance = deposit(2, 64);
	pub const PreimageByteDeposit: Balance = deposit(0, 1);
	pub const PreimageHoldReason: RuntimeHoldReason = RuntimeHoldReason::Preimage(pallet_preimage::HoldReason::Preimage);
}

impl pallet_preimage::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_preimage::weights::SubstrateWeight<Runtime>;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type Consideration = HoldConsideration<
		AccountId,
		Balances,
		PreimageHoldReason,
		LinearStoragePrice<PreimageBaseDeposit, PreimageByteDeposit, Balance>,
	>;
}

impl pallet_insecure_randomness_collective_flip::Config for Runtime {}

parameter_types! {
	//TODO: Set this to a sensible value after testing
	pub const MinSessionLength: BlockNumberFor<Runtime> = 10 as BlockNumberFor<Runtime>;
}

impl pallet_validator_subset_selection::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
	type Randomness = InsecureRandomnessCollectiveFlip;
	type ValidatorSuperset = pallet_nft_staking::SelectableValidators<Runtime>;
	type MinSessionLength = MinSessionLength;
	type SessionHook = (NftDelegation, NftStaking);
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = ValidatorId;
	type ValidatorIdOf = ConvertInto;
	type ShouldEndSession = ValidatorSubsetSelection;
	type NextSessionRotation = ValidatorSubsetSelection;
	type SessionManager = ValidatorSubsetSelection;
	type SessionHandler = <opaque::SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = opaque::SessionKeys;
	type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

// TODO: figure out how to be more generic over the id tuple
impl pallet_offences::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type IdentificationTuple = pallet_im_online::IdentificationTuple<Self>;
	type OnOffenceHandler = NftStaking;
}

#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	Debug,
	MaxEncodedLen,
	scale_info::TypeInfo,
)]
// I made these ProxyTypes according to this: https://mosaicchain.medium.com/account-abstractions-on-mosaic-chain-9b4162897536
pub enum ProxyType {
	/// Allow any kind of transaction, including balance transfers, staking, governance and others on behalf of the proxied account.
	Any,
	/// Allow any type of transaction except the balance transfer functionality.
	/// This proxy does not have permission to access calls in the Balances and XCM pallet.
	NonTransfer,
	/// Allow to make transactions related to only for governance.
	Governance,
	/// Allow all staking-related transactions.
	Staking,
	/// Allow registrars to make judgments on an account’s identity.
	Identity,
	/// Allow to reject and remove any time-delay proxy announcements.
	/// This proxy can only access the “reject_announcement” call from the Proxy pallet.
	Cancel,
}

// Default must be provided and MUST BE the the most permissive value. aka Any
impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}

impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer => {
				matches!(
					c,
					// skipping pallet_balances entirely
					// when xcm will be implemented skip that too

					// As we add more pallets to the chain this needs to be extended as well.
					RuntimeCall::System(..)
						| RuntimeCall::Scheduler(..)
						| RuntimeCall::Timestamp(..)
						| RuntimeCall::Grandpa(..)
						| RuntimeCall::ImOnline(..)
						| RuntimeCall::Proxy(..) | RuntimeCall::NftDelegation(..)
						| RuntimeCall::NftPermission(..)
						| RuntimeCall::NftStaking(..)
						| RuntimeCall::Nfts(..) | RuntimeCall::Session(..)
						| RuntimeCall::Utility(..)
						// Excluding set_recovered(), create_recovery() and initiate_recovery()
						| RuntimeCall::Recovery(pallet_recovery::Call::as_recovered{..})
						| RuntimeCall::Recovery(pallet_recovery::Call::vouch_recovery{..})
						| RuntimeCall::Recovery(pallet_recovery::Call::claim_recovery{..})
						| RuntimeCall::Recovery(pallet_recovery::Call::close_recovery{..})
						| RuntimeCall::Recovery(pallet_recovery::Call::remove_recovery{..})
						| RuntimeCall::Recovery(pallet_recovery::Call::cancel_recovered{..})
						| RuntimeCall::Identity(..)
						| RuntimeCall::CouncilCollective(..)
						| RuntimeCall::CouncilCollectiveMembership(..)
				)
			},
			ProxyType::Governance => {
				matches!(
					c,
					RuntimeCall::Utility(..)
						| RuntimeCall::CouncilCollective(..)
						| RuntimeCall::CouncilCollectiveMembership(..)
				)
			},
			ProxyType::Identity => {
				matches!(c, RuntimeCall::Utility(..) | RuntimeCall::Identity(..))
			},
			ProxyType::Staking => matches!(
				c,
				RuntimeCall::Utility(..) | RuntimeCall::NftStaking(..) | RuntimeCall::Session(..)
			),
			ProxyType::Cancel => {
				matches!(c, RuntimeCall::Proxy(pallet_proxy::Call::reject_announcement { .. }))
			},
		}
	}

	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			(ProxyType::NonTransfer, _) => true,
			_ => false,
		}
	}
}

parameter_types! {
	// went with the documentation on the values
	pub const ProxyDepositBase: Balance = deposit(1, 8);
	pub const ProxyDepositFactor: Balance = deposit(0, 33);
	pub const AnnouncementDepositBase: Balance = deposit(1, 16);
	pub const AnnouncementDepositFactor: Balance = deposit(0, 68);
}

impl pallet_proxy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type WeightInfo = pallet_proxy::weights::SubstrateWeight<Runtime>;
	type CallHasher = BlakeTwo256;
	type MaxProxies = ConstU32<32>;
	type MaxPending = ConstU32<32>;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
}

impl pallet_utility::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	// TODO: review these
	pub const ConfigDepositBase: Balance = 500 * CENTS;
	pub const FriendDepositFactor: Balance = 50 * CENTS;
	pub const MaxFriends: u16 = 9;
	pub const RecoveryDeposit: Balance = 500 * CENTS;
}

impl pallet_recovery::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_recovery::weights::SubstrateWeight<Runtime>;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ConfigDepositBase = ConfigDepositBase;
	type FriendDepositFactor = FriendDepositFactor;
	type MaxFriends = MaxFriends;
	type RecoveryDeposit = RecoveryDeposit;
}

impl pallet_doas::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	// NOTE: changing this to a type which won't check if a vote happened is a risk, because
	// pallet_collective::Call::propose{} with < 2 threshold will result in an immediate pallet_collective::Call::execute{}
	// basically turning all of the council members into a doas account.
	//
	// This ensures that a vote with 2/3 aye ratio is needed for a doas proposal to be accepted.
	type EnsureOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, Council, 2, 3>;
}

parameter_types! {
	/// The time-out for council motions.
	pub const MotionDuration: BlockNumber = MINUTES;
	pub const MaxProposals: u32 = 10;

	// (from docs) NOTE:
	// + Benchmarks will need to be re-run and weights adjusted if this changes.
	// + This pallet assumes that dependents keep to the limit without enforcing it.
	pub const MaxMembers: u32 = 100;
	pub MaxProposalWeight: Weight = sp_runtime::Perbill::from_percent(50) * BlockWeights::get().max_block;
}

type Council = pallet_collective::Instance1;
impl pallet_collective::Config<Council> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	/// The runtime call dispatch type.
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;

	type MotionDuration = MotionDuration;
	type MaxProposals = MaxProposals;
	type MaxMembers = MaxMembers;

	// also check and see which is good for us
	// type DefaultVote = pallet_collective::MoreThanMajorityThenPrimeDefaultVote;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
	type SetMembersOrigin = frame_system::EnsureRoot<AccountId>;
	type MaxProposalWeight = MaxProposalWeight;
}

type EnsureRootOrTwoThirdCouncil = EitherOfDiverse<
	frame_system::EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, Council, 2, 3>,
>;

type CouncilMembership = pallet_membership::Instance1;
impl pallet_membership::Config<CouncilMembership> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AddOrigin = EnsureRootOrTwoThirdCouncil;
	type RemoveOrigin = EnsureRootOrTwoThirdCouncil;
	type SwapOrigin = EnsureRootOrTwoThirdCouncil;
	type ResetOrigin = EnsureRootOrTwoThirdCouncil;
	type PrimeOrigin = EnsureRootOrTwoThirdCouncil;
	type MembershipInitialized = CouncilCollective;
	type MembershipChanged = CouncilCollective;
	type MaxMembers = MaxMembers;
	type WeightInfo = pallet_membership::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
	pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::max_value() / 2;
	pub const MaxAuthorities: u32 = 100;
	pub const MaxKeys: u32 = 10_000;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall,
		public: <Signature as Verify>::Signer,
		account: AccountId,
		nonce: Nonce,
	) -> Option<(
		RuntimeCall,
		<UncheckedExtrinsic as sp_runtime::traits::Extrinsic>::SignaturePayload,
	)> {
		let period =
			BlockHashCount::get().checked_next_power_of_two().map(|c| c / 2).unwrap_or(2) as u64;
		let current_block = System::block_number().saturated_into::<u64>().saturating_sub(1);
		let era = Era::mortal(period, current_block);
		let extra = (
			frame_system::CheckNonZeroSender::<Runtime>::new(),
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(era),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(0),
		);
		let raw_payload = SignedPayload::new(call, extra)
			.map_err(|e| {
				log::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let address = account;
		let (call, extra, _) = raw_payload.deconstruct();

		Some((call, (sp_runtime::MultiAddress::Id(address), signature, extra)))
	}
}

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	RuntimeCall: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = RuntimeCall;
}

type IdTuple = pallet_im_online::IdentificationTuple<Runtime>;
type ImOnlineOffence = pallet_im_online::UnresponsivenessOffence<IdTuple>;

pub struct ImOnlineOffenceAdapter(ImOnlineOffence);

pub struct ImOnlineReporter;

impl Offence<IdTuple> for ImOnlineOffenceAdapter {
	const ID: sp_staking::offence::Kind = *b"mos:imon-offline";
	type TimeSlot = sp_staking::SessionIndex;

	fn offenders(&self) -> Vec<IdTuple> {
		self.0.offenders()
	}

	fn session_index(&self) -> sp_staking::SessionIndex {
		self.0.session_index()
	}

	fn validator_set_count(&self) -> u32 {
		self.0.validator_set_count()
	}

	fn time_slot(&self) -> Self::TimeSlot {
		self.0.time_slot()
	}

	fn disable_strategy(&self) -> sp_staking::offence::DisableStrategy {
		self.0.disable_strategy()
	}

	fn slash_fraction(&self, _offenders: u32) -> Perbill {
		Perbill::from_percent(1)
	}
}

impl ReportOffence<AccountId, IdTuple, ImOnlineOffence> for ImOnlineReporter {
	fn report_offence(
		reporters: Vec<AccountId>,
		offence: ImOnlineOffence,
	) -> Result<(), sp_staking::offence::OffenceError> {
		let offence = ImOnlineOffenceAdapter(offence);
		<Offences as ReportOffence<AccountId, IdTuple, ImOnlineOffenceAdapter>>::report_offence(
			reporters, offence,
		)
	}

	fn is_known_offence(offenders: &[IdTuple], time_slot: &sp_staking::SessionIndex) -> bool {
		<Offences as ReportOffence<AccountId, IdTuple, ImOnlineOffenceAdapter>>::is_known_offence(
			offenders, time_slot,
		)
	}
}

impl pallet_im_online::Config for Runtime {
	type AuthorityId = ImOnlineId;
	type RuntimeEvent = RuntimeEvent;
	type NextSessionRotation = ValidatorSubsetSelection;
	type ValidatorSet = pallet_nft_staking::SlashableValidators<Runtime>;
	type ReportUnresponsiveness = ImOnlineReporter;
	type UnsignedPriority = ImOnlineUnsignedPriority;
	type WeightInfo = pallet_im_online::weights::SubstrateWeight<Runtime>;
	type MaxKeys = MaxKeys;
	type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
	type EventHandler = ImOnline;
}

parameter_types! {
	pub const NftDelegationPalletId: PalletId = PalletId(*b"nft_perm");
}

impl pallet_nft_delegation::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = NftDelegationPalletId;
	type PrivilegedOrigin = frame_system::EnsureRoot<AccountId>;
	type Balance = Balance;
	type NftExpirationHandler = NftStaking;
	type BindMetadata = Self::AccountId;
}

// this is needed, otherwise fmt will remove the :: from ::<Instance1>
#[rustfmt::skip::macros(construct_runtime)]
// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub struct Runtime {
		System: frame_system,
		Timestamp: pallet_timestamp,
		Aura: pallet_aura,
		Grandpa: pallet_grandpa,
		Balances: pallet_balances,
		TransactionPayment: pallet_transaction_payment,
		Nfts: pallet_nfts,
		NftDelegation: pallet_nft_delegation,
		NftPermission: pallet_nft_permission,
		NftStaking: pallet_nft_staking,
		ValidatorSubsetSelection: pallet_validator_subset_selection,
		InsecureRandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip,
		Session: pallet_session,
		Offences: pallet_offences,
		ImOnline: pallet_im_online,
		Authorship: pallet_authorship,
		Proxy: pallet_proxy,
		Utility: pallet_utility,
		Recovery: pallet_recovery,
		Identity: pallet_identity,
		Assets: pallet_assets,
		CouncilCollective: pallet_collective::<Instance1>,
		CouncilCollectiveMembership: pallet_membership::<Instance1>,
		DoAs: pallet_doas,
		Preimage: pallet_preimage,
		Scheduler: pallet_scheduler,
	}
);

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;

/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;

/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	use frame_benchmarking::define_benchmarks;

	define_benchmarks!(
		[frame_benchmarking, BaselineBench::<Runtime>]
		[frame_system, SystemBench::<Runtime>]
		[pallet_balances, Balances]
		[pallet_timestamp, Timestamp]
		[pallet_validator_subset_selection, ValidatorSubsetSelection]
		[pallet_assets, Assets]
		[pallet_proxy, Proxy]
		[pallet_identity, Identity]
		[pallet_utility, Utility]
		[pallet_recovery, Recovery]
		[pallet_collective, CouncilCollective]
		[pallet_membership, CouncilCollectiveMembership]
		[pallet_scheduler, Scheduler]
		[pallet_preimage, Preimage]
	);
}

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) -> ExtrinsicInclusionMode {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> sp_std::vec::Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities().into_inner()
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> sp_consensus_grandpa::AuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn current_set_id() -> sp_consensus_grandpa::SetId {
			Grandpa::current_set_id()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: sp_consensus_grandpa::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			_key_owner_proof: sp_consensus_grandpa::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: sp_consensus_grandpa::SetId,
			_authority_id: GrandpaId,
		) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
			// NOTE: this is the only implementation possible since we've
			// defined our key owner proof type as a bottom type (i.e. a type
			// with no values).
			None
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			let mut list = Vec::<BenchmarkList>::new();

			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();

			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch};
			use sp_storage::TrackedStorageKey;
			use frame_system_benchmarking::Pallet as SystemBench;
			use baseline::Pallet as BaselineBench;

			impl frame_system_benchmarking::Config for Runtime {}
			impl baseline::Config for Runtime {}

			use frame_support::traits::WhitelistedStorageKeys;

			let whitelist: Vec<TrackedStorageKey> = AllPalletsWithSystem::whitelisted_storage_keys();

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);

			add_benchmarks!(params, batches);

			Ok(batches)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here. If any of the pre/post migration checks fail, we shall stop
			// right here and right now.
			let weight = Executive::try_runtime_upgrade(checks).unwrap();

			(weight, BlockWeights::get().max_block)
		}

		fn execute_block(
			block: Block,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).expect("execute-block failed")
		}
	}
}
