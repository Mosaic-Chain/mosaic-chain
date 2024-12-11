use super::*;

use sdk::{
	frame_benchmarking::v2::*,
	// frame_support::{assert_ok, pallet_prelude::*},
	//frame_system::{pallet_prelude::*, RawOrigin},
	frame_system::RawOrigin,
	pallet_balances::{Config as BalancesConfig, Pallet as BalancesPallet},
};

#[expect(clippy::multiple_bound_locations)]
#[benchmarks(where
    <T as Config>::Balance: From<u128>,
	T: BalancesConfig<Balance = <T as Config>::Balance>
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn new_payout() {
		let genesis = GenesisConfig::<T> { incentive_pool: 500u128.into() };
		genesis.build();

		let delegator: T::AccountId = whitelisted_caller();
		let amount = 50u128.into();
		Pallet::<T>::stake_action(&delegator, amount);

		let score_cut = Perbill::from_rational(7u32, 10u32);
		let amount = 50u128.into();
		let origin = RawOrigin::Root;

		#[extrinsic_call]
		_(origin, amount, score_cut);
	}

	#[benchmark]
	fn reset_fuse() {
		#[extrinsic_call]
		_(RawOrigin::Root);
	}

	#[benchmark]
	fn update_and_claim(p: Linear<0, 3>) {
		let genesis = GenesisConfig::<T> { incentive_pool: 500u128.into() };
		genesis.build();

		let caller: T::AccountId = whitelisted_caller();
		<BalancesPallet<T> as Mutate<_>>::mint_into(&caller, 50u128.into())
			.expect("Should succeed");
		Pallet::<T>::stake_action(&caller, 500u128.into());

		let score_cut = Perbill::from_rational(7u32, 10u32);
		let amount: <T as Config>::Balance = 50u128.into();
		let origin = RawOrigin::Root;
		for payout in 0..p {
			<T as Config>::BlockNumberProvider::set_block_number(((payout + 1) * 50_000u32).into());
			Pallet::<T>::new_payout(origin.clone().into(), amount.clone(), score_cut)
				.expect("New payout could be added");
		}

		let origin = RawOrigin::Signed(caller.clone());
		#[extrinsic_call]
		_(origin);
	}
}
