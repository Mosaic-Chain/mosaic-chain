use sdk::{
	frame_support, frame_system, pallet_assets, pallet_aura, pallet_authorship, pallet_balances,
	pallet_identity, pallet_nfts, pallet_offences, pallet_parameters, pallet_preimage,
	pallet_proxy, pallet_recovery, pallet_scheduler, pallet_session, pallet_timestamp,
	pallet_transaction_payment, pallet_utility, sp_core, sp_runtime, sp_staking,
};

use codec::{Compact, Decode, Encode, MaxEncodedLen};
use frame_support::{
	traits::{
		fungible::HoldConsideration, AsEnsureOriginWithArg, InstanceFilter, LinearStoragePrice,
		VariantCountOf,
	},
	weights::{constants::ExtrinsicBaseWeight, ConstantMultiplier, Weight},
};
use pallet_transaction_payment::TargetedFeeAdjustment;
use sp_core::{ConstU128, Get};
use sp_runtime::{
	traits::{BlakeTwo256, Convert, ConvertInto, OpaqueKeys, Verify},
	FixedU128, Perbill, Vec,
};
use sp_staking::offence::{Offence, ReportOffence};

use utils::SessionIndex;

use crate::{
	collectives, opaque, params, AccountId, Aura, AuraId, Balances, FungibleWrapper, HoldVesting,
	ImOnline, ImOnlineId, NftDelegation, NftPermission, NftStaking, Offences, OriginCaller,
	Preimage, Runtime, RuntimeCall, RuntimeEvent, RuntimeFreezeReason, RuntimeHoldReason,
	RuntimeOrigin, Signature, StakingIncentive, System, Treasury, UncheckedExtrinsic,
	ValidatorSubsetSelection,
};
use params::currency::Balance;

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
	type OnChargeTransaction = crate::charge_transaction::ChargeTransaction;
	type OperationalFeeMultiplier = params::constant::transaction_payment::OperationalFeeMultiplier;
	type WeightToFee = WeightToFee;
	type LengthToFee =
		ConstantMultiplier<Balance, ConstU128<{ params::currency::message_fee(0, 1) }>>;
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

	type SessionReward = crate::staking_reward::SessionReward;
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
	type OriginPrivilegeCmp = frame_support::traits::EqualPrivilegeOnly;
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
