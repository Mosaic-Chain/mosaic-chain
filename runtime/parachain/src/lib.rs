// construct_runtime procmacro creates a __hidden_use_of_unchecked_extrinsic type name that rustc frowns upon:
#![allow(non_camel_case_types)]
#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 512.
#![recursion_limit = "512"]

#[allow(clippy::wildcard_imports)]
use sdk::*;

use frame_support::{
	construct_runtime,
	genesis_builder_helper::{build_state, get_preset},
	weights::Weight,
};

use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, Block as BlockT, IdentifyAccount, Verify},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, ExtrinsicInclusionMode, MultiSignature, Vec,
};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::{create_runtime_str, RuntimeVersion};

pub use frame_system::Call as SystemCall;
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::OnChargeTransaction;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill};

pub use pallet_nft_permission;
pub use pallet_validator_subset_selection;

use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use params::currency::Balance;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod charge_transaction;
mod migrations;
pub mod params;
mod weights;

pub mod collectives;
pub mod configs;
pub mod funds;
pub mod staking_reward;
pub mod xcm_config;

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
	use sp_runtime::{generic, impl_opaque_keys};

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
