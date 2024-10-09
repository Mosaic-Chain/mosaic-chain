use super::{
	parameter_types, params, AccountId, Balance, Balances, EnsureWithSuccess, IdentityLookup,
	PalletId, PayFromAccount, Runtime, RuntimeEvent, RuntimeHoldReason,
	UnityAssetBalanceConversion,
};

// Shared values
parameter_types! {
	pub MaxApprovals: u32 = 250;
	pub MaxBalance: Balance = Balance::MAX;
}

macro_rules! impl_fund {
	($fund:ident, $instance:ident, $pallet_id:expr) => {
		pub mod $fund {
			use super::*;

			pub use params::dynamic::$fund::*;
			pub type Instance = pallet_treasury::$instance;

			pub type FundOrigin = pallet_collective::EnsureProportionMoreThan<
				AccountId,
				pallet_collective::$instance,
				1,
				2,
			>;

			parameter_types! {
				pub GetPalletId: PalletId = $pallet_id;
				pub Account: AccountId = pallet_treasury::Pallet::<Runtime, Instance>::account_id();
			}
		}

		impl pallet_treasury::Config<$fund::Instance> for Runtime {
			type Fungible = Balances;
			type ApproveOrigin = $fund::FundOrigin;
			type RejectOrigin = $fund::FundOrigin;
			type RuntimeEvent = RuntimeEvent;
			type RuntimeHoldReason = RuntimeHoldReason;
			type OnSlash = ();
			type ProposalBond = $fund::ProposalBond;
			type ProposalBondMinimum = $fund::ProposalBondMinimum;
			type ProposalBondMaximum = $fund::ProposalBondMaximum;
			type SpendPeriod = $fund::SpendPeriod;
			type Burn = $fund::Burn;
			type PalletId = $fund::GetPalletId;
			type BurnDestination = ();
			type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
			type SpendFunds = ();
			type MaxApprovals = MaxApprovals;
			type SpendOrigin = EnsureWithSuccess<$fund::FundOrigin, AccountId, MaxBalance>;
			type AssetKind = ();
			type Beneficiary = AccountId;
			type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
			type Paymaster = PayFromAccount<Balances, $fund::Account>;
			type BalanceConverter = UnityAssetBalanceConversion;
			type PayoutPeriod = $fund::PayoutPeriod;
			#[cfg(feature = "runtime-benchmarks")]
			type BenchmarkHelper = ();
		}
	};
}

impl_fund!(treasury, Instance1, PalletId(*b"1reasury"));
impl_fund!(development_fund, Instance2, PalletId(*b"2devfund"));
impl_fund!(financial_fund, Instance3, PalletId(*b"3finfund"));
impl_fund!(community_fund, Instance4, PalletId(*b"4comfund"));
impl_fund!(team_and_advisors_fund, Instance5, PalletId(*b"5eamfund"));
impl_fund!(security_fund, Instance6, PalletId(*b"6secfund"));
impl_fund!(education_fund, Instance7, PalletId(*b"7edufund"));
