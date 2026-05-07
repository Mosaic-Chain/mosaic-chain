use sdk::{
	assets_common, cumulus_pallet_aura_ext, cumulus_pallet_xcmp_queue, cumulus_primitives_core,
	cumulus_primitives_storage_weight_reclaim, frame_support, frame_system,
	pallet_asset_conversion, pallet_asset_conversion_tx_payment, pallet_asset_rate, pallet_assets,
	pallet_balances, pallet_message_queue, pallet_transaction_payment, parachains_common,
	polkadot_runtime_common, sp_core, sp_runtime, staging_parachain_info, staging_xcm as xcm,
};

use assets_common::{
	foreign_creators::ForeignCreators,
	local_and_foreign_assets::{ForeignAssetReserveData, LocalFromLeft, TargetFromLeft},
	matching::FromSiblingParachain,
	AssetIdForPoolAssets, AssetIdForTrustBackedAssetsConvert,
};
use codec::{Compact, Encode};
use cumulus_pallet_parachain_system::RelayNumberMonotonicallyIncreases;
use cumulus_primitives_core::{AggregateMessageOrigin, ParaId};
use frame_support::{
	derive_impl,
	traits::{
		fungible, fungibles, tokens::imbalance::ResolveAssetTo, NeverEnsureOrigin, TransformOrigin,
	},
	weights::constants::RocksDbWeight,
};
use parachains_common::{
	message_queue::{NarrowOriginToSibling, ParaIdToSibling},
	AssetIdForTrustBackedAssets,
};
use sp_core::{ConstU128, ConstU32};
use sp_runtime::{generic::Era, SaturatedConversion};
use xcm::latest::prelude::*;

use crate::{
	collectives::{self, CollectiveMajority, CollectiveSuperMajority},
	funds::treasury::Account as TreasuryAccount,
	params, weights,
	xcm_config::{
		self, AssetsPalletLocation, HereLocation, LocationToAccountId,
		XcmOriginToTransactDispatchOrigin,
	},
	AccountId, Assets, Balance, Balances, Block, ForeignAssets, MessageQueue, PalletInfo,
	ParachainInfo, ParachainSystem, PoolAssets, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin,
	RuntimeTask, SignedPayload, System, UncheckedExtrinsic, XcmpQueue,
};

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

	type SystemWeightInfo = weights::pallet::frame_system::Weights<Runtime>;
}

pub type ForeignAssetsInstance = pallet_assets::Instance2;
impl pallet_assets::Config<ForeignAssetsInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = Location;
	type AssetIdParameter = Location;
	type Currency = Balances;
	type CreateOrigin = ForeignCreators<
		(
			FromSiblingParachain<ParachainInfo, Location>,
			/*
			TODO: Consider these
			FromNetwork<
				xcm_config::UniversalLocation,
				xcm_config::bridging::to_ethereum::EthereumNetwork,
				Location,
			>,
			xcm_config::bridging::to_kusama::KusamaAssetFromAssetHubKusama,
			*/
		),
		LocationToAccountId,
		AccountId,
		Location,
	>;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type Freezer = ();
	type Holder = ();
	type Extra = ();
	type CallbackHandle = ();
	type ReserveData = ForeignAssetReserveData;

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = assets_common::benchmarks::LocationAssetsBenchmarkHelper;

	type AssetDeposit = params::dynamic::assets::AssetDeposit;
	type AssetAccountDeposit = params::dynamic::assets::AssetAccountDeposit;
	type MetadataDepositBase = params::dynamic::assets::MetadataDepositBase;
	type MetadataDepositPerByte = params::dynamic::assets::MetadataDepositPerByte;
	type ApprovalDeposit = params::dynamic::assets::ApprovalDeposit;

	type StringLimit = params::constant::assets::StringLimit;
	type RemoveItemsLimit = params::constant::assets::RemoveItemsLimit;
	type WeightInfo = weights::pallet::assets::Weights<Runtime>;
}

pub type PoolAssetsInstance = pallet_assets::Instance3;
impl pallet_assets::Config<PoolAssetsInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = AssetIdForPoolAssets;
	type AssetIdParameter = Compact<AssetIdForPoolAssets>;
	type Currency = Balances;
	type CreateOrigin = NeverEnsureOrigin<AccountId>;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type Freezer = ();
	type Holder = ();
	type Extra = ();
	type CallbackHandle = ();
	type ReserveData = ();

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();

	type AssetDeposit = ConstU128<0>;
	type AssetAccountDeposit = params::dynamic::assets::AssetAccountDeposit;
	type MetadataDepositBase = ConstU128<0>;
	type MetadataDepositPerByte = ConstU128<0>;
	type ApprovalDeposit = params::dynamic::assets::ApprovalDeposit;

	type StringLimit = params::constant::assets::StringLimit;
	type RemoveItemsLimit = params::constant::assets::RemoveItemsLimit;
	type WeightInfo = weights::pallet::assets::Weights<Runtime>;
}

/// Union fungibles implementation for `Assets`` and `ForeignAssets`.
pub type LocalAndForeignAssets = fungibles::UnionOf<
	Assets,
	ForeignAssets,
	LocalFromLeft<
		AssetIdForTrustBackedAssetsConvert<AssetsPalletLocation, Location>,
		AssetIdForTrustBackedAssets,
		Location,
	>,
	Location,
	AccountId,
>;

/// Union fungibles implementation for [`LocalAndForeignAssets`] and `Balances`.
pub type NativeAndAssets = fungible::UnionOf<
	Balances,
	LocalAndForeignAssets,
	TargetFromLeft<HereLocation, Location>,
	Location,
	AccountId,
>;

pub type PoolAssetKind = Location;
pub type PoolId = (PoolAssetKind, PoolAssetKind);
pub type PoolIdToAccountId = pallet_asset_conversion::AccountIdConverter<
	params::constant::asset_conversion::PalletId,
	PoolId,
>;

impl pallet_asset_conversion::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type HigherPrecisionBalance = sp_core::U256;
	type AssetKind = PoolAssetKind;
	type Assets = NativeAndAssets;
	type PoolId = PoolId;
	type PoolLocator = pallet_asset_conversion::WithFirstAsset<
		HereLocation,
		AccountId,
		PoolAssetKind,
		PoolIdToAccountId,
	>;
	type PoolAssetId = u32;
	type PoolAssets = PoolAssets;
	type PoolSetupFee = params::dynamic::asset_conversion::PoolSetupFee;
	type PoolSetupFeeAsset = HereLocation;
	type PoolSetupFeeTarget = ResolveAssetTo<TreasuryAccount, Self::Assets>;
	type LiquidityWithdrawalFee = params::constant::asset_conversion::LiquidityWithdrawalFee;
	type LPFee = ConstU32<3>;
	type PalletId = params::constant::asset_conversion::PalletId;
	type MaxSwapPathLength = ConstU32<3>;
	type MintMinLiquidity = ConstU128<100>;
	type WeightInfo = (); //weights::pallet::asset_conversion::WeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = assets_common::benchmarks::AssetPairFactory<
		HereLocation,
		ParachainInfo,
		xcm_config::AssetsPalletIndex,
		PoolAssetKind,
	>;
}

impl pallet_asset_conversion_tx_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type AssetId = Location;
	type OnChargeAssetTransaction = crate::charge_asset_transaction::ChargeAssetTransaction;
	type WeightInfo = (); // TODO: weights::pallet_asset_conversion_tx_payment::WeightInfo<Self>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = AssetConversionTxHelper;
}

#[cfg(feature = "runtime-benchmarks")]
pub struct AssetConversionTxHelper;

#[cfg(feature = "runtime-benchmarks")]
pub type AssetConversionAssetIdFor<T> = <T as pallet_asset_conversion_tx_payment::Config>::AssetId;

#[cfg(feature = "runtime-benchmarks")]
impl
	pallet_asset_conversion_tx_payment::BenchmarkHelperTrait<
		AccountId,
		AssetConversionAssetIdFor<Runtime>,
		AssetConversionAssetIdFor<Runtime>,
	> for AssetConversionTxHelper
{
	fn create_asset_id_parameter(
		seed: u32,
	) -> (AssetConversionAssetIdFor<Runtime>, AssetConversionAssetIdFor<Runtime>) {
		// Use a different parachain' foreign assets pallet so that the asset is indeed foreign.
		let asset_id = Location::new(
			1,
			[
				Junction::Parachain(3000),
				Junction::PalletInstance(53),
				Junction::GeneralIndex(seed.into()),
			],
		);
		(asset_id.clone(), asset_id)
	}

	fn setup_balances_and_pool(asset_id: AssetConversionAssetIdFor<Runtime>, account: AccountId) {
		use crate::AssetConversion;
		use alloc::boxed::Box;
		use frame_support::{assert_ok, traits::fungibles::Mutate};

		assert_ok!(ForeignAssets::force_create(
			RuntimeOrigin::root(),
			asset_id.clone(),
			account.clone().into(), /* owner */
			true,                   /* is_sufficient */
			1,
		));

		let lp_provider = account.clone();
		use frame_support::traits::Currency;
		let _ = Balances::deposit_creating(&lp_provider, u64::MAX.into());
		assert_ok!(ForeignAssets::mint_into(asset_id.clone(), &lp_provider, u64::MAX.into()));

		let token_native = Box::new(HereLocation::get());
		let token_second = Box::new(asset_id);

		assert_ok!(AssetConversion::create_pool(
			RuntimeOrigin::signed(lp_provider.clone()),
			token_native.clone(),
			token_second.clone()
		));

		assert_ok!(AssetConversion::add_liquidity(
			RuntimeOrigin::signed(lp_provider.clone()),
			token_native,
			token_second,
			(u32::MAX / 8).into(), // 1 desired
			u32::MAX.into(),       // 2 desired
			1,                     // 1 min
			1,                     // 2 min
			lp_provider,
		));
	}
}

impl pallet_asset_rate::Config for Runtime {
	type RuntimeEvent = <Runtime as cumulus_pallet_parachain_system::Config>::RuntimeEvent;

	type Currency = Balances;
	type AssetKind = Location;

	type CreateOrigin =
		CollectiveSuperMajority<collectives::financial_collective::CollectiveInstance>;
	type RemoveOrigin = Self::CreateOrigin;
	type UpdateOrigin = CollectiveMajority<collectives::financial_collective::CollectiveInstance>;
	type WeightInfo = (); // TODO: benchmark

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmarks::AssetRateArguments;
}

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarks {
	use super::*;
	use core::marker::PhantomData;
	use frame_support::traits::Get;
	use pallet_asset_rate::AssetKindFactory;
	use pallet_treasury::ArgumentsFactory as TreasuryArgumentsFactory;
	use sp_core::{ConstU32, ConstU8};

	/// Provides a factory method for the [`VersionedLocatableAsset`].
	/// The location of the asset is determined as a Parachain with an ID equal to the passed seed.
	pub struct AssetRateArguments;
	impl AssetKindFactory<Location> for AssetRateArguments {
		fn create_asset_kind(seed: u32) -> Location {
			Location::new(
				0,
				[
					Parachain(seed),
					PalletInstance(seed.try_into().unwrap()),
					GeneralIndex(seed.into()),
				],
			)
		}
	}

	/// Provide factory methods for the [`Location`] and the `Beneficiary` of the
	/// [`VersionedLocation`]. The location of the asset is determined as a Parachain with an
	/// ID equal to the passed seed.
	pub struct TreasuryArguments<Parents = ConstU8<0>, ParaId = ConstU32<0>>(
		PhantomData<(Parents, ParaId)>,
	);
	impl<Parents: Get<u8>, ParaId: Get<u32>> TreasuryArgumentsFactory<Location, Location>
		for TreasuryArguments<Parents, ParaId>
	{
		fn create_asset_kind(seed: u32) -> Location {
			Location::new(
				Parents::get(),
				[
					Parachain(ParaId::get()),
					PalletInstance(seed.try_into().unwrap()),
					GeneralIndex(seed.into()),
				],
			)
		}
		fn create_beneficiary(seed: [u8; 32]) -> Location {
			Location::new(0, [AccountId32 { network: None, id: seed }])
		}
	}
}
impl staging_parachain_info::Config for Runtime {}

impl pallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	#[cfg(feature = "runtime-benchmarks")]
	type MessageProcessor = pallet_message_queue::mock_helpers::NoopMessageProcessor<
		cumulus_primitives_core::AggregateMessageOrigin,
	>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MessageProcessor = sdk::staging_xcm_builder::ProcessXcmMessage<
		AggregateMessageOrigin,
		sdk::staging_xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
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
	type WeightInfo = weights::pallet::message_queue::Weights<Runtime>;
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
	type PriceForSiblingDelivery = polkadot_runtime_common::xcm_sender::ExponentialPrice<
		params::constant::xcmp_queue::FeeAssetId,
		params::constant::xcmp_queue::BaseDeliveryFee,
		params::constant::xcmp_queue::MessageByteFee,
		XcmpQueue,
	>;
	type MaxActiveOutboundChannels = params::constant::xcmp_queue::MaxActiveOutboundChannels;
	type MaxPageSize = params::constant::xcmp_queue::MaxPageSize;
	type WeightInfo = weights::pallet::xcmp_queue::Weights<Runtime>;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
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
	type WeightInfo = weights::pallet::parachain_system::Weights<Runtime>;

	// TODO: consider setting it to a slightly higher value to help reduce forks on the parachain side.
	type RelayParentOffset = ConstU32<0>;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_signed_transaction<
		C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>,
	>(
		call: RuntimeCall,
		public: Self::Public,
		account: Self::AccountId,
		nonce: Self::Nonce,
	) -> Option<Self::Extrinsic> {
		let period = Self::BlockHashCount::get()
			.checked_next_power_of_two()
			.map(|c| c / 2)
			.unwrap_or(2) as u64;
		let current_block = System::block_number().saturated_into::<u64>().saturating_sub(1);
		let era = Era::mortal(period, current_block);

		#[expect(deprecated, reason = "not using cumulus-pallet-weight-reclaim yet")]
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
				log::warn!("Unable to create signed payload: {e:?}");
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let (call, tx_ext, _) = raw_payload.deconstruct();
		let address = account.into();
		let transaction = UncheckedExtrinsic::new_signed(call, address, signature, tx_ext);
		Some(transaction)
	}
}
