// construct_runtime procmacro creates a __hidden_use_of_unchecked_extrinsic type name that rustc frowns upon:
#![allow(non_camel_case_types)]
#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 512.
#![recursion_limit = "512"]

#[allow(clippy::wildcard_imports)]
use sdk::*;

extern crate alloc;

use frame_support::{
	genesis_builder_helper::{build_state, get_preset},
	weights::Weight,
};

use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, Block as BlockT, IdentifyAccount, NumberFor, Verify},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, ExtrinsicInclusionMode, MultiSignature, Vec,
};

#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::{Cow, RuntimeVersion};

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
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;

#[cfg(test)]
#[path = "../../parachain/src/mock.rs"]
mod mock;

#[cfg(test)]
#[path = "../../parachain/src/tests/mod.rs"]
mod tests;

#[path = "../../parachain/src/charge_transaction.rs"]
mod charge_transaction;

#[path = "../../parachain/src/collectives.rs"]
pub mod collectives;

#[path = "../../parachain/src/funds.rs"]
pub mod funds;

#[path = "../../parachain/src/staking_reward.rs"]
pub mod staking_reward;

pub mod configs;
pub mod params;

mod weights;

// NOTE: The v101 migrations of the parachain uses feature flags that don't exist for the solochain.
#[path = "../../parachain/src/migrations/v103.rs"]
mod migrations;

use params::currency::Balance;

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
			pub grandpa: Grandpa,
			pub im_online: ImOnline,
		}
	}
}

// To learn more about runtime versioning, see:
// https://docs.substrate.io/main-docs/build/upgrade#runtime-versioning
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: Cow::Borrowed("mosaic-testnet-solo"),
	impl_name: Cow::Borrowed("mosaic-testnet-solo"),
	authoring_version: 1,
	// The version of the runtime specification. A full node will not attempt to use its native
	//   runtime in substitute for the on-chain Wasm runtime unless all of `spec_name`,
	//   `spec_version`, and `authoring_version` are the same between Wasm and native.
	// This value is set to 100 to notify Polkadot-JS App (https://polkadot.js.org/apps) to use
	//   the compatible custom types.
	spec_version: 104,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 2,
	system_version: 1,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;

/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// The SignedExtension to the basic transaction logic.
pub type TxExtension = (
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
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;

/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, TxExtension>;

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
	migrations::MigrateFromV101<Runtime>,
>;

#[frame_support::runtime]
mod runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask
	)]
	pub struct Runtime;

	#[runtime::pallet_index(0)]
	pub type System = frame_system;
	#[runtime::pallet_index(2)]
	pub type Parameters = pallet_parameters;
	#[runtime::pallet_index(3)]
	pub type Timestamp = pallet_timestamp;
	#[runtime::pallet_index(5)]
	pub type Aura = pallet_aura;
	#[runtime::pallet_index(6)]
	pub type Grandpa = pallet_grandpa;
	#[runtime::pallet_index(7)]
	pub type Balances = pallet_balances;
	#[runtime::pallet_index(8)]
	pub type TransactionPayment = pallet_transaction_payment;
	#[runtime::pallet_index(9)]
	pub type Nfts = pallet_nfts;
	#[runtime::pallet_index(10)]
	pub type NftDelegation = pallet_nft_delegation;
	#[runtime::pallet_index(11)]
	pub type NftPermission = pallet_nft_permission;
	#[runtime::pallet_index(12)]
	pub type NftStaking = pallet_nft_staking;
	#[runtime::pallet_index(13)]
	pub type ValidatorSubsetSelection = pallet_validator_subset_selection;
	#[runtime::pallet_index(14)]
	pub type Session = pallet_session;
	#[runtime::pallet_index(15)]
	pub type Offences = pallet_offences;
	#[runtime::pallet_index(16)]
	pub type ImOnline = pallet_im_online;
	#[runtime::pallet_index(17)]
	pub type Authorship = pallet_authorship;
	#[runtime::pallet_index(18)]
	pub type Proxy = pallet_proxy;
	#[runtime::pallet_index(19)]
	pub type Utility = pallet_utility;
	#[runtime::pallet_index(20)]
	pub type Recovery = pallet_recovery;
	#[runtime::pallet_index(21)]
	pub type Identity = pallet_identity;
	#[runtime::pallet_index(22)]
	pub type Assets = pallet_assets;
	#[runtime::pallet_index(23)]
	pub type DoAs = pallet_doas;
	#[runtime::pallet_index(24)]
	pub type Preimage = pallet_preimage;
	#[runtime::pallet_index(25)]
	pub type Scheduler = pallet_scheduler;
	#[runtime::pallet_index(26)]
	pub type CouncilCollective = pallet_collective<Instance1>;
	#[runtime::pallet_index(27)]
	pub type CouncilMembership = pallet_membership<Instance1>;
	#[runtime::pallet_index(28)]
	pub type DevelopmentCollective = pallet_collective<Instance2>;
	#[runtime::pallet_index(29)]
	pub type DevelopmentMembership = pallet_membership<Instance2>;
	#[runtime::pallet_index(30)]
	pub type FinancialCollective = pallet_collective<Instance3>;
	#[runtime::pallet_index(31)]
	pub type FinancialMembership = pallet_membership<Instance3>;
	#[runtime::pallet_index(32)]
	pub type CommunityCollective = pallet_collective<Instance4>;
	#[runtime::pallet_index(33)]
	pub type CommunityMembership = pallet_membership<Instance4>;
	#[runtime::pallet_index(34)]
	pub type TeamAndAdvisorsCollective = pallet_collective<Instance5>;
	#[runtime::pallet_index(35)]
	pub type TeamAndAdvisorsMembership = pallet_membership<Instance5>;
	#[runtime::pallet_index(36)]
	pub type SecurityCollective = pallet_collective<Instance6>;
	#[runtime::pallet_index(37)]
	pub type SecurityMembership = pallet_membership<Instance6>;
	#[runtime::pallet_index(38)]
	pub type EducationCollective = pallet_collective<Instance7>;
	#[runtime::pallet_index(39)]
	pub type EducationMembership = pallet_membership<Instance7>;
	#[runtime::pallet_index(40)]
	pub type Treasury = pallet_treasury<Instance1>;
	#[runtime::pallet_index(41)]
	pub type DevelopmentFund = pallet_treasury<Instance2>;
	#[runtime::pallet_index(42)]
	pub type FinancialFund = pallet_treasury<Instance3>;
	#[runtime::pallet_index(43)]
	pub type CommunityFund = pallet_treasury<Instance4>;
	#[runtime::pallet_index(44)]
	pub type TeamAndAdvisorsFund = pallet_treasury<Instance5>;
	#[runtime::pallet_index(45)]
	pub type SecurityFund = pallet_treasury<Instance6>;
	#[runtime::pallet_index(46)]
	pub type EducationFund = pallet_treasury<Instance7>;
	#[runtime::pallet_index(47)]
	pub type Airdrop = pallet_airdrop;
	#[runtime::pallet_index(48)]
	pub type HoldVesting = pallet_hold_vesting;
	#[runtime::pallet_index(49)]
	pub type VestingToFreeze = pallet_vesting_to_freeze;
	#[runtime::pallet_index(50)]
	pub type StakingIncentive = pallet_staking_incentive;
	#[runtime::pallet_index(51)]
	pub type FungibleWrapper = pallet_extra_fungible_events;
}

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	use super::*;
	use frame_benchmarking::define_benchmarks;

	define_benchmarks!(
		[frame_system, SystemBench::<Runtime>]
		[pallet_parameters, Parameters]
		[pallet_timestamp, Timestamp]
		[pallet_balances, Balances]
		[pallet_transaction_payment, TransactionPayment]
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
		[pallet_session, SessionBench::<Runtime>]
		[pallet_collective, CouncilCollective]
		[pallet_membership, CouncilMembership]
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
			use frame_benchmarking::BenchmarkList;
			use frame_support::traits::StorageInfoTrait;
			use frame_system_benchmarking::Pallet as SystemBench;
			use cumulus_pallet_session_benchmarking::Pallet as SessionBench;

			let mut list = Vec::<BenchmarkList>::new();

			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();

			(list, storage_info)
		}

		#[allow(non_local_definitions)]
		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, alloc::string::String> {
			use frame_benchmarking::BenchmarkBatch;
			use sp_storage::TrackedStorageKey;
			use frame_system_benchmarking::Pallet as SystemBench;

			// NOTE: we use the implementation from cumulus as we can satisfy
			// its bounds in contrast with `pallet_session_benchmarking`.
			use cumulus_pallet_session_benchmarking::Pallet as SessionBench;

			impl frame_system_benchmarking::Config for Runtime {}
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

			(weight, <Runtime as frame_system::Config>::BlockWeights::get().max_block)
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
