use sdk::{
	frame_support, frame_system, pallet_balances, pallet_grandpa, pallet_transaction_payment,
	sp_core, sp_runtime,
};

use codec::Encode;
use frame_support::{derive_impl, weights::constants::RocksDbWeight};
use sp_runtime::{generic::Era, traits::Verify, SaturatedConversion};

use crate::{
	params, weights, AccountId, Block, Nonce, PalletInfo, Runtime, RuntimeCall, RuntimeEvent,
	RuntimeOrigin, RuntimeTask, Signature, SignedPayload, System, UncheckedExtrinsic,
};
use params::currency::Balance;

#[path = "../../../parachain/src/configs/common.rs"]
mod common;
pub use common::*;

// Configure FRAME pallets to include in runtime.
#[derive_impl(frame_system::config_preludes::SolochainDefaultConfig)]
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

	type SystemWeightInfo = weights::pallet::frame_system::Weights<Runtime>;
}

impl pallet_grandpa::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MaxAuthorities = params::constant::grandpa::MaxAuthorities;
	type MaxSetIdSessionEntries = params::constant::grandpa::MaxSetIdSessionEntries;
	type KeyOwnerProof = sp_core::Void;
	type EquivocationReportSystem = ();
	type MaxNominators = params::constant::grandpa::MaxNominators;

	// We do not currently benchmark `grandpa` as it's default implementation seems to be good enough
	// and the weights don't depend so much on the other parts of the runtime.
	//
	// TODO: for the sake of completeness benchmark this as well.
	type WeightInfo = ();
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
