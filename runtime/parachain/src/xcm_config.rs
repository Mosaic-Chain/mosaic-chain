use sdk::{
	assets_common, cumulus_pallet_xcm, cumulus_primitives_core, cumulus_primitives_utility,
	frame_support, frame_system, pallet_xcm, parachains_common, polkadot_parachain_primitives,
	sp_runtime, staging_xcm as xcm, staging_xcm_builder as xcm_builder,
	staging_xcm_executor as xcm_executor,
};

use super::{
	charge_asset_transaction::AssetConverter, configs::parachain::NativeAndAssets,
	funds::treasury::Account as TreasuryAccount, AccountId, AllPalletsWithSystem, AssetConversion,
	Assets, Balance, Balances, ForeignAssets, ParachainInfo, ParachainSystem, PolkadotXcm,
	PoolAssets, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin, XcmpQueue,
};

use assets_common::{
	matching::{
		IsForeignConcreteAsset, NonTeleportableAssetFromTrustedReserve,
		TeleportableAssetWithTrustedReserve,
	},
	TrustBackedAssetsAsLocation, TrustBackedAssetsConvertedConcreteId,
};
use frame_support::{
	parameter_types,
	traits::{
		tokens::imbalance::{ResolveAssetTo, ResolveTo},
		ConstU32, Contains, Disabled, Equals, Everything, EverythingBut, Nothing, PalletInfoAccess,
	},
	weights::{IdentityFee, Weight},
};

use cumulus_primitives_core::ParaId;
use frame_system::EnsureRoot;
use pallet_xcm::XcmPassthrough;
use parachains_common::xcm_config::{AllSiblingSystemParachains, RelayOrOtherSystemParachains};
use polkadot_parachain_primitives::primitives::Sibling;
use sp_runtime::traits::TryConvertInto;
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowExplicitUnpaidExecutionFrom, AllowTopLevelPaidExecutionFrom,
	DenyReserveTransferToRelayChain, DenyThenTry, EnsureXcmOrigin, FixedWeightBounds,
	FrameTransactionalProcessor, FungibleAdapter, FungiblesAdapter, IsConcrete, LocalMint,
	MatchedConvertedConcreteId, MintLocation, NoChecking, ParentIsPreset, RelayChainAsNative,
	SendXcmFeeToAccount, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SingleAssetExchangeAdapter,
	SovereignSignedViaLocation, StartsWith, StartsWithExplicitGlobalConsensus, TakeWeightCredit,
	TrailingSetTopicAsId, UsingComponents, WithComputedOrigin, WithLatestLocationConverter,
	WithUniqueTopic, XcmFeeManagerFromComponents,
};
use xcm_executor::XcmExecutor;

parameter_types! {
	pub const RelayLocation: Location = Location::parent();
	// NOTE: paseo also uses `NetworkId::Polkadot`
	pub const RelayNetworkId: NetworkId = NetworkId::Polkadot;
	pub const RelayNetwork: Option<NetworkId> = Some(RelayNetworkId::get());
	pub const HereLocation: Location = Location::here();
	pub AssetsPalletIndex: u8 = <Assets as PalletInfoAccess>::index() as u8;
	pub AssetsPalletLocation: Location = PalletInstance(AssetsPalletIndex::get()).into();
	pub PoolAssetsPalletIndex: u8 = <PoolAssets as PalletInfoAccess>::index() as u8;
	pub PoolAssetsPalletLocation: Location = PalletInstance(PoolAssetsPalletIndex::get()).into();
	pub AssetHubLocation: Location = Location::new(1, Parachain(1000));
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	pub SelfParaId: ParaId = ParachainInfo::parachain_id();
	pub UniversalLocation: InteriorLocation = [GlobalConsensus(RelayNetworkId::get()), Parachain(SelfParaId::get().into())].into();
	pub CheckingAccount: AccountId = PolkadotXcm::check_account();
	pub TeleportTracking: Option<(AccountId, MintLocation)> = Some((CheckingAccount::get(), MintLocation::Local));

	pub DotLocation: Location = RelayLocation::get();
	pub UsdtLocation: Location = Location::new(
			1,
			[
				Parachain(1000),
				PalletInstance(50),
				GeneralIndex(1984)
			],
		);
	pub UsdcLocation: Location = Location::new(
			1,
			[
				Parachain(1000),
				PalletInstance(50),
				GeneralIndex(1337)
			],
		);
}

/// Locations that will not be charged fees in the executor,
/// either execution or delivery.
/// We only waive fees for system functions, which these locations represent.
pub type WaivedLocations =
	(RelayOrOtherSystemParachains<AllSiblingSystemParachains, Runtime>, Equals<HereLocation>);

/// Type for specifying how a `Location` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the parent `AccountId`.
	ParentIsPreset<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<RelayNetwork, AccountId>,
);

/// Means for transacting assets on this chain.
pub type NativeTransactor = FungibleAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<HereLocation>,
	// Do a simple punn to convert an AccountId32 Location into a native chain account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We track teleports.
	TeleportTracking,
>;

/// Means for transacting assets besides the native currency on this chain.
pub type AssetsTransactor = FungiblesAdapter<
	// Use this fungibles implementation:
	Assets,
	// Use this currency when it is a fungible asset matching the given location or name:
	TrustBackedAssetsConvertedConcreteId<AssetsPalletLocation, Balance>,
	// Convert an XCM `Location` into a local account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We only want to allow teleports of known assets. We use non-zero issuance as an indication
	// that this asset is known.
	LocalMint<parachains_common::impls::NonZeroIssuance<AccountId, Assets>>,
	// The account to use for tracking teleports.
	CheckingAccount,
>;

/// Means for transacting foreign assets from different global consensus.
pub type ForeignAssetsTransactor = FungiblesAdapter<
	// Use this fungibles implementation:
	ForeignAssets,
	// Use this currency when it is a fungible asset matching the given location or name:
	ForeignAssetsConvertedConcreteId,
	// Convert an XCM `Location` into a local account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We dont need to check teleports here.
	NoChecking,
	// The account to use for tracking teleports.
	CheckingAccount,
>;

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
	// Native converter for Relay-chain (Parent) location; will convert to a `Relay` origin when
	// recognized.
	RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
	// Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
	// recognized.
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `RuntimeOrigin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<RuntimeOrigin>,
);

parameter_types! {
	// One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
	// TODO: benchmark
	pub UnitWeightCost: Weight = Weight::from_parts(1_000_000_000, 64 * 1024);
	pub const MaxInstructions: u32 = 100;
	pub const MaxAssetsIntoHolding: u32 = 64;
}

pub struct ParentOrParentsExecutivePlurality;
impl Contains<Location> for ParentOrParentsExecutivePlurality {
	fn contains(location: &Location) -> bool {
		matches!(location.unpack(), (1, [] | [Plurality { id: BodyId::Executive, .. }]))
	}
}

pub type Barrier = TrailingSetTopicAsId<
	DenyThenTry<
		DenyReserveTransferToRelayChain,
		(
			TakeWeightCredit,
			WithComputedOrigin<
				(
					AllowTopLevelPaidExecutionFrom<Everything>,
					AllowExplicitUnpaidExecutionFrom<ParentOrParentsExecutivePlurality>,
					// ^^^ Parent and its exec plurality get free execution
				),
				UniversalLocation,
				ConstU32<8>,
			>,
		),
	>,
>;

pub struct OnlyNative;
impl Contains<(Location, crate::Vec<Asset>)> for OnlyNative {
	fn contains((_, assets): &(Location, crate::Vec<Asset>)) -> bool {
		assets.iter().all(|asset| {
			log::trace!(target: "xcm::OnlyNative", "Asset to be sent out: {asset:?}");
			if let Asset { id: asset_id, fun: Fungible(_) } = asset {
				asset_id.0 == HereLocation::get()
			} else {
				false
			}
		})
	}
}

impl frame_support::traits::ContainsPair<Asset, Location> for OnlyNative {
	fn contains(asset: &Asset, location: &Location) -> bool {
		log::debug!(target: "xcm::OnlyNative", "Asset to be sent out: {asset:?}, location: {location:?}");
		// Mosaic chain and AssetHub is a trusted reserve of MOS
		matches!(asset, Asset { id: asset_id, fun: Fungible(_) } if asset_id.0 == HereLocation::get()
			&& (location == &HereLocation::get() || location == &AssetHubLocation::get()))
	}
}

pub type TrustedReserves = (
	OnlyNative,
	IsForeignConcreteAsset<
		NonTeleportableAssetFromTrustedReserve<SelfParaId, crate::ForeignAssets>,
	>,
);

pub type TrustedTeleporters = (
	OnlyNative,
	// Can teleport sibling's assets back-and-forth according to their trusted reserves.
	// (teleportable when `Here` and `origin` are both trusted reserve locations)
	IsForeignConcreteAsset<TeleportableAssetWithTrustedReserve<SelfParaId, crate::ForeignAssets>>,
);

/// `AssetId`/`Balance` converter for `ForeignAssets`
pub type ForeignAssetsConvertedConcreteId = MatchedConvertedConcreteId<
	Location,
	Balance,
	EverythingBut<(
		// Here we rely on fact that something like this works:
		// assert!(Location::new(1, [Parachain(100)]).starts_with(&Location::parent()));
		// assert!([Parachain(100)].into().starts_with(&Here));
		StartsWith<HereLocation>,
		StartsWith<AssetsPalletLocation>,
		// Ignore assets that start explicitly with our `GlobalConsensus(NetworkId)`, means:
		// - foreign assets from our consensus should be: `Location {parents: 1, X*(Parachain(xyz),..)}`
		// - foreign assets outside our consensus with the same `GlobalConsensus(NetworkId)` won't be accepted here
		StartsWithExplicitGlobalConsensus<RelayNetworkId>,
	)>,
	WithLatestLocationConverter<Location>,
	TryConvertInto,
>;

pub type ConvertibleAssetsMatcher = (
	TrustBackedAssetsAsLocation<AssetsPalletLocation, Balance, Location>,
	ForeignAssetsConvertedConcreteId,
);

/// Asset converter for pool assets.
/// Used to convert one asset to another, when there is a pool available between the two.
/// This type thus allows paying delivery fees with any asset as long as there is a pool between
/// said asset and the asset required for fee payment.
pub type PoolAssetsExchanger = SingleAssetExchangeAdapter<
	AssetConversion,
	NativeAndAssets,
	ConvertibleAssetsMatcher,
	AccountId,
>;

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	// How to withdraw and deposit an asset.
	type AssetTransactor = (NativeTransactor, AssetsTransactor, ForeignAssetsTransactor);
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type IsReserve = TrustedReserves;
	type IsTeleporter = TrustedTeleporters;
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type Trader = (
		UsingComponents<
			IdentityFee<Balance>,
			HereLocation,
			AccountId,
			Balances,
			ResolveTo<TreasuryAccount, Balances>,
		>,
		// This trader allows to pay with any assets exchangeable to MOS with
		// [`AssetConverter`].
		cumulus_primitives_utility::SwapFirstAssetTrader<
			HereLocation,
			AssetConverter,
			IdentityFee<Balance>,
			NativeAndAssets,
			ConvertibleAssetsMatcher,
			ResolveAssetTo<TreasuryAccount, NativeAndAssets>,
			AccountId,
		>,
	);
	type ResponseHandler = PolkadotXcm;
	type AssetTrap = PolkadotXcm;
	type AssetClaims = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type AssetLocker = ();
	type AssetExchanger = PoolAssetsExchanger;
	type FeeManager = XcmFeeManagerFromComponents<
		WaivedLocations,
		SendXcmFeeToAccount<Self::AssetTransactor, TreasuryAccount>,
	>;
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Everything;
	type Aliasers = Nothing;
	type TransactionalProcessor = FrameTransactionalProcessor;
	type HrmpNewChannelOpenRequestHandler = ();
	type HrmpChannelAcceptedHandler = ();
	type HrmpChannelClosingHandler = ();
	type XcmRecorder = PolkadotXcm;
	type XcmEventEmitter = PolkadotXcm;
}

/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = WithUniqueTopic<(
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, (), ()>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
)>;

impl pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Nothing;
	// ^ Disable dispatchable execute on the XCM pallet.
	// Needs to be `Everything` for local testing.
	type XcmExecutor = XcmExecutor<XcmConfig>;
	// NOTE: `TrustedTeleporters` and `TrustedReserves` already filter these
	type XcmTeleportFilter = Everything;
	type XcmReserveTransferFilter = Everything;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type UniversalLocation = UniversalLocation;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;

	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	// ^ Override for AdvertisedXcmVersion default
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type TrustedLockers = ();
	type SovereignAccountOf = LocationToAccountId;
	type MaxLockers = ConstU32<8>;
	type WeightInfo = pallet_xcm::TestWeightInfo; // Configure based on benchmarking results.
	type AdminOrigin = EnsureRoot<AccountId>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
	type AuthorizedAliasConsideration = Disabled;
}

impl cumulus_pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}
