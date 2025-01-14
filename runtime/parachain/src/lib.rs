// construct_runtime procmacro creates a __hidden_use_of_unchecked_extrinsic type name that rustc frowns upon:
#![allow(non_camel_case_types)]
#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 512.
#![recursion_limit = "512"]

#[allow(clippy::wildcard_imports)]
use sdk::*;

use sp_std::prelude::*;

use codec::{Compact, Decode, Encode, MaxEncodedLen};
use frame_support::{
	derive_impl,
	genesis_builder_helper::{build_state, get_preset},
	pallet_prelude::TransactionValidityError,
	traits::{
		fungible::{Balanced, Credit, Debt, HoldConsideration, Inspect},
		tokens::{PayFromAccount, Precision, UnityAssetBalanceConversion},
		AsEnsureOriginWithArg, Currency, EitherOfDiverse, EqualPrivilegeOnly, Imbalance,
		InstanceFilter, LinearStoragePrice, TransformOrigin, VariantCountOf,
	},
	weights::ConstantMultiplier,
	PalletId,
};
use frame_system::EnsureWithSuccess;
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::KeyTypeId, Get, OpaqueMetadata};
use sp_runtime::{
	create_runtime_str,
	generic::{self, Era},
	impl_opaque_keys,
	traits::{
		BlakeTwo256, Block as BlockT, Convert, ConvertInto, IdentifyAccount, IdentityLookup,
		OpaqueKeys, Verify, Zero,
	},
	transaction_validity::InvalidTransaction,
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, ExtrinsicInclusionMode, FixedU128, MultiSignature, SaturatedConversion,
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
use pallet_transaction_payment::{OnChargeTransaction, TargetedFeeAdjustment};
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill};

pub use pallet_nft_permission;
pub use pallet_validator_subset_selection;

use pallet_im_online::sr25519::AuthorityId as ImOnlineId;

use cumulus_pallet_parachain_system::RelayNumberMonotonicallyIncreases;
use cumulus_primitives_core::{AggregateMessageOrigin, ParaId};
use parachains_common::message_queue::{NarrowOriginToSibling, ParaIdToSibling};

use params::currency::{message_fee, Balance};

use utils::SessionIndex;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod charge_transaction;
mod migrations;
pub mod params;
mod weights;

pub mod collectives;
pub mod funds;
pub mod staking_reward;
pub mod xcm_config;

use xcm_config::XcmOriginToTransactDispatchOrigin;

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

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
			pub im_online: ImOnline,
		}
	}
}

// To learn more about runtime versioning, see:
// https://docs.substrate.io/main-docs/build/upgrade#runtime-versioning
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("mosaic-chain"),
	impl_name: create_runtime_str!("mosaic-chain"),
	authoring_version: 1,
	// The version of the runtime specification. A full node will not attempt to use its native
	//   runtime in substitute for the on-chain Wasm runtime unless all of `spec_name`,
	//   `spec_version`, and `authoring_version` are the same between Wasm and native.
	// This value is set to 100 to notify Polkadot-JS App (https://polkadot.js.org/apps) to use
	//   the compatible custom types.
	spec_version: 101,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 2,
	state_version: 1,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

// Configure FRAME pallets to include in runtime.
#[derive_impl(frame_system::config_preludes::ParaChainDefaultConfig)]
impl frame_system::Config for Runtime {
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = params::constant::system::BlockWeights;

	/// The maximum length of a block (in bytes).
	type BlockLength = params::constant::system::BlockLength;

	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = params::constant::system::BlockHashCount;

	/// The block type for the runtime
	type Block = Block;

	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight; // TODO: what should we really use here?

	/// Version of the runtime.
	type Version = params::constant::system::Version;

	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;

	/// This is used as an identifier of the chain.
	type SS58Prefix = params::constant::system::SS58Prefix;

	/// The action to take on a Runtime Upgrade
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
}

impl pallet_parameters::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeParameters = params::RuntimeParameters;
	type AdminOrigin = collectives::CouncilOrigin;
	type WeightInfo = ();
}

impl pallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = u32;
	type AssetIdParameter = Compact<u32>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId>>;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type Freezer = ();
	type Extra = ();
	type CallbackHandle = ();

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();

	type AssetDeposit = params::dynamic::assets::AssetDeposit;
	type AssetAccountDeposit = params::dynamic::assets::AssetAccountDeposit;
	type MetadataDepositBase = params::dynamic::assets::MetadataDepositBase;
	type MetadataDepositPerByte = params::dynamic::assets::MetadataDepositPerByte;
	type ApprovalDeposit = params::dynamic::assets::ApprovalDeposit;

	type StringLimit = params::constant::assets::StringLimit;
	type RemoveItemsLimit = params::constant::assets::RemoveItemsLimit;
	type WeightInfo = pallet_assets::weights::SubstrateWeight<Self>;
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = params::constant::aura::MaxActiveAuthorities;
	type AllowMultipleBlocksPerSlot = params::constant::aura::AllowMultipleBlocksPerSlot;
	type SlotDuration = params::constant::aura::SlotDuration;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = params::constant::timestamp::MinimumPeriod;
	type WeightInfo = ();
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = params::constant::balances::MaxLocks;
	type MaxFreezes = VariantCountOf<RuntimeFreezeReason>;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxReserves = params::constant::balances::MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = RuntimeHoldReason;

	/// The type for recording an account's balance.
	type Balance = Balance;

	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = params::constant::balances::ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Self>;
}

impl pallet_extra_fungible_events::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Fungible = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
}

pub struct WeightToFee;
impl frame_support::weights::WeightToFee for WeightToFee {
	type Balance = Balance;

	fn weight_to_fee(weight: &Weight) -> Self::Balance {
		let time_fee = weight.ref_time() as u128 * params::currency::CENTS
			/ (2000 * Balance::from(ExtrinsicBaseWeight::get().ref_time()));
		let pov_fee = weight.proof_size() as u128 * params::currency::CENTS / 200_000;

		time_fee.max(pov_fee)
	}
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = charge_transaction::ChargeTransaction;
	type OperationalFeeMultiplier = params::constant::transaction_payment::OperationalFeeMultiplier;
	type WeightToFee = WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, ConstU128<{ message_fee(0, 1) }>>;
	type FeeMultiplierUpdate = TargetedFeeAdjustment<
		Self,
		params::constant::transaction_payment::TargetBlockFullness,
		params::constant::transaction_payment::AdjustmentVariable,
		params::constant::transaction_payment::MinimumMultiplier,
		params::constant::transaction_payment::MaximumMultiplier,
	>;
}

impl pallet_nfts::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId>>;
	type Locker = ();
	type CollectionDeposit = params::dynamic::nfts::CollectionDeposit;
	type ItemDeposit = params::dynamic::nfts::ItemDeposit;
	type MetadataDepositBase = params::dynamic::nfts::MetadataDepositBase;
	type AttributeDepositBase = params::dynamic::nfts::AttributeDepositBase;
	type DepositPerByte = params::dynamic::nfts::DepositPerByte;
	type StringLimit = params::constant::nfts::StringLimit;
	type KeyLimit = params::constant::nfts::KeyLimit;
	type ValueLimit = params::constant::nfts::ValueLimit;
	type ApprovalsLimit = params::constant::nfts::ApprovalsLimit;
	type ItemAttributesApprovalsLimit = params::constant::nfts::ItemAttributesApprovalsLimit;
	type MaxTips = params::constant::nfts::MaxTips;
	type MaxDeadlineDuration = params::constant::nfts::MaxDeadlineDuration;
	type MaxAttributesPerCall = params::constant::nfts::MaxAttributesPerCall;
	type Features = ();
	type OffchainSignature = Signature;
	type OffchainPublic = <Signature as Verify>::Signer;
	type WeightInfo = pallet_nfts::weights::SubstrateWeight<Self>;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
}

impl pallet_nft_permission::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type PalletId = params::constant::nft_permission::PalletId;
	type PrivilegedOrigin = frame_system::EnsureRoot<AccountId>;
	type Permission = pallet_nft_staking::PermissionType;
	type WeightInfo = pallet_nft_permission::weights::SubstrateWeight<Self>;
}

pub struct IdTupleToValidatorId;

impl Convert<IdTuple, AccountId> for IdTupleToValidatorId {
	fn convert(id_tuple: IdTuple) -> AccountId {
		id_tuple.0
	}
}

#[cfg(feature = "runtime-benchmarks")]
pub struct NftStakingBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pallet_nft_staking::BenchmarkHelper<Runtime> for NftStakingBenchmarkHelper {
	fn id_tuple_from_account(acc: AccountId) -> IdTuple {
		(acc.clone(), acc)
	}
}

impl pallet_nft_staking::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Fungible = FungibleWrapper;
	type NftDelegationHandler = NftDelegation;
	type NftStakingHandler = NftPermission;
	type Balance = Balance;
	type ItemId = <Self as pallet_nfts::Config>::ItemId;
	type OffenderToValidatorId = IdTupleToValidatorId;

	type SlackingPeriod = params::dynamic::nft_staking::SlackingPeriod;
	type NominalValueThreshold = params::dynamic::nft_staking::NominalValueThreshold;
	type MinimumStakingPeriod = params::dynamic::nft_staking::MinimumStakingPeriod;
	type MinimumCommissionRate = params::dynamic::nft_staking::MinimumCommission;
	type MinimumStakingAmount = params::dynamic::nft_staking::MinimumStakingAmount;
	type MaximumStakePercentage = params::dynamic::nft_staking::MaximumStakePercentage;

	type SessionReward = staking_reward::SessionReward;
	type MaximumContractsPerValidator = params::constant::nft_staking::MaximumContractsPerValidator;
	type MaximumBoundValidators = params::constant::nft_staking::MaximumBoundValidators;
	type ContributionPercentage = params::dynamic::nft_staking::ContributionPercentage;
	type ContributionDestination = Treasury;
	type Hooks = StakingIncentive;
	type WeightInfo = pallet_nft_staking::weights::SubstrateWeight<Self>;

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = NftStakingBenchmarkHelper;
}

impl pallet_identity::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type Slashed = ();
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type RegistrarOrigin = frame_system::EnsureRoot<AccountId>;
	type MaxSubAccounts = params::constant::identity::MaxSubAccounts;
	type MaxRegistrars = params::constant::identity::MaxRegistrars;
	type MaxSuffixLength = params::constant::identity::MaxSuffixLength;
	type MaxUsernameLength = params::constant::identity::MaxUsernameLength;
	type IdentityInformation =
		pallet_identity::legacy::IdentityInfo<params::constant::identity::MaxAdditionalFields>;
	type OffchainSignature = Signature;
	type SigningPublicKey = <Signature as Verify>::Signer;
	type UsernameAuthorityOrigin = frame_system::EnsureRoot<AccountId>;
	type PendingUsernameExpiration = params::constant::identity::PendingUsernameExpiration;

	type ByteDeposit = params::dynamic::identity::ByteDeposit;
	type BasicDeposit = params::dynamic::identity::BasicDeposit;
	type SubAccountDeposit = params::dynamic::identity::SubAccountDeposit;
	type WeightInfo = pallet_identity::weights::SubstrateWeight<Self>;
}

impl pallet_scheduler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = params::constant::scheduler::MaximumWeight;
	type ScheduleOrigin = collectives::CouncilOrigin;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type MaxScheduledPerBlock = params::constant::scheduler::MaxScheduledPerBlock;
	type Preimages = Preimage;
	type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Self>;
}

impl pallet_preimage::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = collectives::CouncilOrigin;
	type Consideration = HoldConsideration<
		AccountId,
		FungibleWrapper,
		params::constant::preimage::HoldReason,
		LinearStoragePrice<
			params::dynamic::preimage::BaseDeposit,
			params::dynamic::preimage::ByteDeposit,
			Balance,
		>,
	>;
	type WeightInfo = pallet_preimage::weights::SubstrateWeight<Self>;
}

impl pallet_validator_subset_selection::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
	type ValidatorSuperset = pallet_nft_staking::SelectableValidators<Self>;
	type SubsetSize = params::dynamic::validator_subset_selection::SubsetSize;
	type MinSessionLength = params::dynamic::validator_subset_selection::MinSessionLength;
	// NftStaking assumes that expiring delegator NFTs have already expired.
	type SessionHook = (NftDelegation, NftStaking);
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
	type ValidatorIdOf = ConvertInto;
	type ShouldEndSession = ValidatorSubsetSelection;
	type NextSessionRotation = ValidatorSubsetSelection;
	type SessionManager = ValidatorSubsetSelection;
	type SessionHandler = <opaque::SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = opaque::SessionKeys;
	type WeightInfo = pallet_session::weights::SubstrateWeight<Self>;
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
// We made these ProxyTypes according to this: https://mosaicchain.medium.com/account-abstractions-on-mosaic-chain-9b4162897536
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
	/// Allow registrars to make judgments on an account's identity.
	Identity,
	/// Allow to reject and remove any time-delay proxy announcements.
	/// This proxy can only access the `reject_announcement` call from the Proxy pallet.
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
		fn is_governance(c: &RuntimeCall) -> bool {
			matches!(
				c,
				RuntimeCall::CouncilCollective(..)
					| RuntimeCall::CouncilMembership(..)
					| RuntimeCall::DevelopmentCollective(..)
					| RuntimeCall::DevelopmentMembership(..)
					| RuntimeCall::FinancialCollective(..)
					| RuntimeCall::FinancialMembership(..)
					| RuntimeCall::CommunityCollective(..)
					| RuntimeCall::CommunityMembership(..)
					| RuntimeCall::TeamAndAdvisorsCollective(..)
					| RuntimeCall::TeamAndAdvisorsMembership(..)
					| RuntimeCall::SecurityCollective(..)
					| RuntimeCall::SecurityMembership(..)
					| RuntimeCall::EducationCollective(..)
					| RuntimeCall::EducationMembership(..)
			)
		}

		fn nft_non_transfer(c: &RuntimeCall) -> bool {
			let RuntimeCall::Nfts(x) = c else {
				return false;
			};

			matches!(
				x,
				pallet_nfts::Call::create { .. }
					| pallet_nfts::Call::force_create { .. }
					| pallet_nfts::Call::mint { .. }
					| pallet_nfts::Call::force_mint { .. }
					| pallet_nfts::Call::lock_item_transfer { .. }
					| pallet_nfts::Call::lock_collection { .. }
					| pallet_nfts::Call::mint_pre_signed { .. }
			)
		}

		if let ProxyType::Cancel = self {
			return matches!(c, RuntimeCall::Proxy(pallet_proxy::Call::reject_announcement { .. }));
		};

		if let RuntimeCall::Utility(..) = c {
			return true;
		}

		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer => {
				matches!(
					c,
					// skipping pallet_balances entirely
					// when xcm will be implemented skip that too

					// As we add more pallets to the chain this needs to be extended as well.
					RuntimeCall::System(..)
						| RuntimeCall::Timestamp(..)
						| RuntimeCall::ImOnline(..)
						| RuntimeCall::NftDelegation(..)
						| RuntimeCall::NftPermission(..)
						| RuntimeCall::NftStaking(..)
						| RuntimeCall::Session(..)
						| RuntimeCall::Identity(..)
				) || nft_non_transfer(c)
					|| is_governance(c)
			},
			ProxyType::Governance => is_governance(c),
			ProxyType::Identity => matches!(c, RuntimeCall::Identity(..)),
			ProxyType::Staking => {
				matches!(c, RuntimeCall::NftStaking(..) | RuntimeCall::Session(..))
			},
			ProxyType::Cancel => unreachable!("Handled already"),
		}
	}

	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			(ProxyType::NonTransfer, x) => *x != ProxyType::Cancel,
			_ => false,
		}
	}
}

impl pallet_proxy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type CallHasher = BlakeTwo256;
	type MaxProxies = params::constant::proxy::MaxProxies;
	type MaxPending = params::constant::proxy::MaxPending;
	type ProxyDepositBase = params::dynamic::proxy::DepositBase;
	type ProxyDepositFactor = params::dynamic::proxy::DepositFactor;
	type AnnouncementDepositBase = params::dynamic::proxy::AnnouncementDepositBase;
	type AnnouncementDepositFactor = params::dynamic::proxy::AnnouncementDepositFactor;
	type WeightInfo = pallet_proxy::weights::SubstrateWeight<Self>;
}

impl pallet_utility::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = pallet_utility::weights::SubstrateWeight<Self>;
}

impl pallet_recovery::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type MaxFriends = params::constant::recovery::MaxFriends;
	type ConfigDepositBase = params::dynamic::recovery::ConfigDepositBase;
	type FriendDepositFactor = params::dynamic::recovery::FriendDepositFactor;
	type RecoveryDeposit = params::dynamic::recovery::RecoveryDeposit;
	type WeightInfo = pallet_recovery::weights::SubstrateWeight<Self>;
}

impl pallet_doas::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type EnsureOrigin = collectives::CouncilOrigin;
	type WeightInfo = pallet_doas::weights::SubstrateWeight<Self>;
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
		let period = Self::BlockHashCount::get()
			.checked_next_power_of_two()
			.map(|c| c / 2)
			.unwrap_or(2) as u64;
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
			cumulus_primitives_storage_weight_reclaim::StorageWeightReclaim::<Runtime>::new(),
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
	type TimeSlot = SessionIndex;

	fn offenders(&self) -> Vec<IdTuple> {
		self.0.offenders()
	}

	fn session_index(&self) -> SessionIndex {
		self.0.session_index()
	}

	fn validator_set_count(&self) -> u32 {
		self.0.validator_set_count()
	}

	fn time_slot(&self) -> Self::TimeSlot {
		self.0.time_slot()
	}

	fn slash_fraction(&self, _offenders: u32) -> Perbill {
		params::dynamic::tokenomics::SlashFraction::get()
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

	fn is_known_offence(offenders: &[IdTuple], time_slot: &SessionIndex) -> bool {
		<Offences as ReportOffence<AccountId, IdTuple, ImOnlineOffenceAdapter>>::is_known_offence(
			offenders, time_slot,
		)
	}
}

impl pallet_im_online::Config for Runtime {
	type AuthorityKey = ImOnlineId;
	type RuntimeEvent = RuntimeEvent;
	type ValidatorSet = pallet_nft_staking::SlashableValidators<Self>;
	type ReportUnresponsiveness = ImOnlineReporter;
	type UnsignedPriority = params::constant::im_online::UnsignedPriority;
	type WeightInfo = pallet_im_online::weights::SubstrateWeight<Self>;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
	type EventHandler = ImOnline;
}

pub struct CurrentSession;
impl sp_core::Get<SessionIndex> for CurrentSession {
	fn get() -> SessionIndex {
		pallet_session::CurrentIndex::<Runtime>::get()
	}
}

impl pallet_nft_delegation::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = params::constant::nft_delegation::PalletId;
	type MaxExpirationsPerSession = params::constant::nft_delegation::MaxExpirationsPerSession;
	type CurrentSession = CurrentSession;
	type PrivilegedOrigin = collectives::CouncilOrigin;
	type Balance = Balance;
	type NftExpirationHandler = NftStaking;
	type BindMetadata = AccountId;
	type WeightInfo = pallet_nft_delegation::weights::SubstrateWeight<Self>;
}

impl pallet_airdrop::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type PermissionType = pallet_nft_staking::PermissionType;
	type ItemId = <Self as pallet_nfts::Config>::ItemId;
	type NftPermission = NftPermission;
	type Fungible = FungibleWrapper;
	type DelegatorNftBindMetadata = AccountId;
	type NftDelegation = NftDelegation;
	type VestingSchedule = HoldVesting;
	type BaseTransactionPriority = params::constant::airdrop::BaseTransactionPriority;
	type MaxAirdropsInPool = params::constant::airdrop::MaxAirdropsInPool;
	const MAX_DELEGATOR_NFTS: u32 = params::constant::airdrop::MAX_DELEGATOR_NFTS;
	type WeightInfo = pallet_airdrop::weights::SubstrateWeight<Self>;
}

impl pallet_hold_vesting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Balance = Balance;
	type Fungible = FungibleWrapper;
	type BlockNumberToBalance = ConvertInto;
	type MinVestedTransfer = params::constant::hold_vesting::MinVestedTransfer;
	type BlockNumberProvider = System;
	const MAX_VESTING_SCHEDULES: u32 = params::constant::hold_vesting::MAX_VESTING_SCHEDULES;
	type WeightInfo = pallet_hold_vesting::weights::SubstrateWeight<Self>;
}

impl pallet_vesting_to_freeze::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type Balance = Balance;
	type Fungible = FungibleWrapper;
	type VestingSchedule = HoldVesting;
	type BlockNumberToBalance = ConvertInto;
	type BlockNumberProvider = System;
	type MaxFrozenSchedules = params::constant::vesting_to_freeze::MaxFrozenSchedules;
	type MaxFreezes = params::constant::vesting_to_freeze::MaxFreezes;
	type MaxVestingSchedules = params::constant::vesting_to_freeze::MaxVestingSchedules;
	type WeightInfo = pallet_vesting_to_freeze::SubstrateWeight<Self>;
}

pub struct BalanceToScore;

impl Convert<Balance, FixedU128> for BalanceToScore {
	fn convert(a: Balance) -> FixedU128 {
		// One MOS is 10^18 tiles
		// The divisor of `FixedU128` is also 10^18
		// => 1 MOS can be considered as 1 Score
		// => Around 3.4028 * 10^20 MOS can thus be represented as score
		FixedU128::from_inner(a)
	}
}

impl pallet_staking_incentive::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type Fungible = FungibleWrapper;
	type VestingSchedule = HoldVesting;
	type ClaimVestingScheduleLength =
		params::constant::staking_incentive::ClaimVestingScheduleLength;
	type PerBlockMultiplier = params::constant::staking_incentive::PerBlockMultiplier;
	type BlockNumberProvider = System;
	type BalanceToScore = BalanceToScore;
	type PalletId = params::constant::staking_incentive::PalletId;
	type MaxPayouts = params::constant::staking_incentive::MaxPayouts;
	type WeightInfo = pallet_staking_incentive::weights::SubstrateWeight<Self>;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
	type WeightInfo = (); // Configure based on benchmarking results.
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type SelfParaId = ParachainInfo;
	type OutboundXcmpMessageSource = XcmpQueue;
	type DmpQueue = frame_support::traits::EnqueueWithOrigin<
		MessageQueue,
		params::constant::parachain_system::RelayOrigin,
	>;
	type ReservedDmpWeight = params::constant::parachain_system::ReservedDmpWeight;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = params::constant::parachain_system::ReservedXcmpWeight;
	type CheckAssociatedRelayNumber = RelayNumberMonotonicallyIncreases;
	type ConsensusHook = cumulus_pallet_aura_ext::FixedVelocityConsensusHook<
		Self,
		{ params::constant::parachain_system::RELAY_CHAIN_SLOT_DURATION_MILLIS },
		{ params::constant::parachain_system::BLOCK_PROCESSING_VELOCITY },
		{ params::constant::parachain_system::UNINCLUDED_SEGMENT_CAPACITY },
	>;
}

impl staging_parachain_info::Config for Runtime {}

impl pallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = (); // Configure based on benchmarking results.
	#[cfg(feature = "runtime-benchmarks")]
	type MessageProcessor = pallet_message_queue::mock_helpers::NoopMessageProcessor<
		cumulus_primitives_core::AggregateMessageOrigin,
	>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MessageProcessor = staging_xcm_builder::ProcessXcmMessage<
		AggregateMessageOrigin,
		staging_xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
		RuntimeCall,
	>;
	type Size = u32;
	// The XCMP queue pallet is only ever able to handle the `Sibling(ParaId)` origin:
	type QueueChangeHandler = NarrowOriginToSibling<XcmpQueue>;
	type QueuePausedQuery = NarrowOriginToSibling<XcmpQueue>;
	type HeapSize = params::constant::message_queue::HeapSize;
	type MaxStale = params::constant::message_queue::MaxStale;
	type ServiceWeight = params::constant::message_queue::ServiceWeight;
	type IdleMaxServiceWeight = params::constant::message_queue::ServiceWeight;
}

impl cumulus_pallet_aura_ext::Config for Runtime {}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ChannelInfo = ParachainSystem;
	type VersionWrapper = ();
	// Enqueue XCMP messages from siblings for later processing.
	type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
	type MaxInboundSuspended = params::constant::xcmp_queue::MaxInboundSuspended;
	type ControllerOrigin = collectives::CouncilOrigin;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type WeightInfo = (); // Configure based on benchmarking results.
	type PriceForSiblingDelivery = polkadot_runtime_common::xcm_sender::ExponentialPrice<
		params::constant::xcmp_queue::FeeAssetId,
		params::constant::xcmp_queue::BaseDeliveryFee,
		params::constant::xcmp_queue::MessageByteFee,
		XcmpQueue,
	>;
	type MaxActiveOutboundChannels = params::constant::xcmp_queue::MaxActiveOutboundChannels;
	type MaxPageSize = params::constant::xcmp_queue::MaxPageSize;
}

// this is needed, otherwise fmt will remove the :: from ::<Instance1>
#[rustfmt::skip::macros(construct_runtime)]
// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub struct Runtime {
		System: frame_system,
		ParachainSystem: cumulus_pallet_parachain_system,
		Parameters: pallet_parameters,
		Timestamp: pallet_timestamp,
		ParachainInfo: staging_parachain_info,
		Aura: pallet_aura,
		AuraExt: cumulus_pallet_aura_ext,
		Balances: pallet_balances,
		TransactionPayment: pallet_transaction_payment,
		Nfts: pallet_nfts,
		NftDelegation: pallet_nft_delegation,
		NftPermission: pallet_nft_permission,
		NftStaking: pallet_nft_staking,
		ValidatorSubsetSelection: pallet_validator_subset_selection,
		Session: pallet_session,
		Offences: pallet_offences,
		ImOnline: pallet_im_online,
		Authorship: pallet_authorship,
		Proxy: pallet_proxy,
		Utility: pallet_utility,
		Recovery: pallet_recovery,
		Identity: pallet_identity,
		Assets: pallet_assets,
		DoAs: pallet_doas,
		Preimage: pallet_preimage,
		Scheduler: pallet_scheduler,
		CouncilCollective: pallet_collective::<Instance1>,
		CouncilMembership: pallet_membership::<Instance1>,
		DevelopmentCollective: pallet_collective::<Instance2>,
		DevelopmentMembership: pallet_membership::<Instance2>,
		FinancialCollective: pallet_collective::<Instance3>,
		FinancialMembership: pallet_membership::<Instance3>,
		CommunityCollective: pallet_collective::<Instance4>,
		CommunityMembership: pallet_membership::<Instance4>,
		TeamAndAdvisorsCollective: pallet_collective::<Instance5>,
		TeamAndAdvisorsMembership: pallet_membership::<Instance5>,
		SecurityCollective: pallet_collective::<Instance6>,
		SecurityMembership: pallet_membership::<Instance6>,
		EducationCollective: pallet_collective::<Instance7>,
		EducationMembership: pallet_membership::<Instance7>,
		Treasury: pallet_treasury::<Instance1>,
		DevelopmentFund: pallet_treasury::<Instance2>,
		FinancialFund: pallet_treasury::<Instance3>,
		CommunityFund: pallet_treasury::<Instance4>,
		TeamAndAdvisorsFund: pallet_treasury::<Instance5>,
		SecurityFund: pallet_treasury::<Instance6>,
		EducationFund: pallet_treasury::<Instance7>,
		Airdrop: pallet_airdrop,
		HoldVesting: pallet_hold_vesting,
		VestingToFreeze: pallet_vesting_to_freeze,
		StakingIncentive: pallet_staking_incentive,
		FungibleWrapper: pallet_extra_fungible_events,
		XcmpQueue: cumulus_pallet_xcmp_queue,
		PolkadotXcm: pallet_xcm,
		CumulusXcm: cumulus_pallet_xcm,
		MessageQueue: pallet_message_queue,
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
	cumulus_primitives_storage_weight_reclaim::StorageWeightReclaim<Runtime>,
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
	migrations::v101::MigrateV100ToV101<Runtime>,
>;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	use super::*;
	use frame_benchmarking::define_benchmarks;

	define_benchmarks!(
		[frame_benchmarking, SystemBench::<Runtime>]
		[frame_system, SystemBench::<Runtime>]
		[cumulus_pallet_parachain_system, ParachainSystem]
		[cumulus_pallet_xcmp_queue, XcmpQueue]
		[pallet_message_queue, MessageQueue]
		[pallet_parameters, Parameters]
		[pallet_timestamp, Timestamp]
		[pallet_balances, Balances]
		[pallet_nfts, Nfts]
		[pallet_nft_delegation, NftDelegation]
		[pallet_nft_permission, NftPermission]
		[pallet_nft_staking, NftStaking]
		[pallet_validator_subset_selection, ValidatorSubsetSelection]
		[pallet_im_online, ImOnline]
		[pallet_proxy, Proxy]
		[pallet_utility, Utility]
		[pallet_recovery, Recovery]
		[pallet_identity, Identity]
		[pallet_assets, Assets]
		[pallet_doas, DoAs]
		[pallet_preimage, Preimage]
		[pallet_scheduler, Scheduler]
		[pallet_collective, CouncilCollective]
		[pallet_membership, CouncilMembership]
		[pallet_collective, DevelopmentCollective]
		[pallet_membership, DevelopmentMembership]
		[pallet_collective, FinancialCollective]
		[pallet_membership, FinancialMembership]
		[pallet_collective, CommunityCollective]
		[pallet_membership, CommunityMembership]
		[pallet_collective, TeamAndAdvisorsCollective]
		[pallet_membership, TeamAndAdvisorsMembership]
		[pallet_collective, SecurityCollective]
		[pallet_membership, SecurityMembership]
		[pallet_collective, EducationCollective]
		[pallet_membership, EducationMembership]
		[pallet_treasury, Treasury]
		[pallet_airdrop, Airdrop]
		[pallet_hold_vesting, HoldVesting]
		[pallet_vesting_to_freeze, VestingToFreeze]
		[pallet_staking_incentive, StakingIncentive]
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
			pallet_aura::Authorities::<Runtime>::get().into_inner()
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

	impl cumulus_primitives_aura::AuraUnincludedSegmentApi<Block> for Runtime {
		fn can_build_upon(
			included_hash: <Block as BlockT>::Hash,
			slot: cumulus_primitives_aura::Slot,
		) -> bool {
			<Runtime as cumulus_pallet_parachain_system::Config>::ConsensusHook::can_build_upon(included_hash, slot)
		}
	}

	impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
			ParachainSystem::collect_collation_info(header)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();
			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{Benchmarking, BenchmarkBatch, BenchmarkError};
			use sp_storage::TrackedStorageKey;
			use frame_system_benchmarking::Pallet as SystemBench;

			impl frame_system_benchmarking::Config for Runtime {
				fn setup_set_code_requirements(code: &sp_std::vec::Vec<u8>) -> Result<(), BenchmarkError> {
					ParachainSystem::initialize_for_set_code_benchmark(code.len() as u32);
					Ok(())
				}

				fn verify_set_code() {
					System::assert_last_event(cumulus_pallet_parachain_system::Event::<Runtime>::ValidationFunctionStored.into());
				}
			}

			impl cumulus_pallet_session_benchmarking::Config for Runtime {}

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

			(weight, params::constant::system::BlockWeights::get().max_block)
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

	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
			get_preset::<RuntimeGenesisConfig>(id, |_| None)
		}

		fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
			Default::default()
		}
	}
}

cumulus_pallet_parachain_system::register_validate_block! {
	Runtime = Runtime,
	BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
}
