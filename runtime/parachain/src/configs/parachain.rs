use sdk::{
	cumulus_pallet_aura_ext, cumulus_pallet_xcmp_queue, cumulus_primitives_core,
	cumulus_primitives_storage_weight_reclaim, frame_support, frame_system, pallet_balances,
	pallet_message_queue, pallet_transaction_payment, parachains_common, polkadot_runtime_common,
	sp_core, sp_runtime, staging_parachain_info,
};

use codec::Encode;
use cumulus_pallet_parachain_system::RelayNumberMonotonicallyIncreases;
use cumulus_primitives_core::{AggregateMessageOrigin, ParaId};
use frame_support::{derive_impl, traits::TransformOrigin, weights::constants::RocksDbWeight};
use parachains_common::message_queue::{NarrowOriginToSibling, ParaIdToSibling};
use sp_core::ConstU32;
use sp_runtime::{generic::Era, SaturatedConversion};

use crate::{
	collectives, params, weights, xcm_config::XcmOriginToTransactDispatchOrigin, Balance, Block,
	MessageQueue, PalletInfo, ParachainInfo, ParachainSystem, Runtime, RuntimeCall, RuntimeEvent,
	RuntimeOrigin, RuntimeTask, SignedPayload, System, UncheckedExtrinsic, XcmpQueue,
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
		sdk::staging_xcm_executor::XcmExecutor<crate::xcm_config::XcmConfig>,
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

	type SelectCore = cumulus_pallet_parachain_system::DefaultCoreSelector<Runtime>;

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
