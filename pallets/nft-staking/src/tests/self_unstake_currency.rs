use super::*;

#[rstest]
fn dpos_self_unstake_currency_is_successful(mut ext: TestExternalities) {
	let permission = PermissionType::DPoS;
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let _ = EndowParams::default().account_id(validator.account_id).endow();

		Staking::self_stake_currency(validator.origin.clone(), 500).expect("could stake currency");

		skip_min_staking_period();

		let res = Staking::self_unstake_currency(validator.origin, 300);
		assert_ok!(res, ());

		assert_current_validator_stake!(
			&validator.account_id,
			Some(TotalValidatorStake { total_stake, .. }) if total_stake == 300
		);

		assert_current_contract!(&validator.account_id, &validator.account_id,
			Some(Contract {
				stake: Stake {
					currency,
					permission_nft: Some(NOMINAL_VALUE),
					..
				},
				..
			}) if currency == 200);

		System::assert_last_event(
			Event::<Test>::CurrencyUnstaked {
				validator: validator.account_id,
				staker: validator.account_id,
				amount: 300,
			}
			.into(),
		);

		next_session();

		// pallet_balances does not have a Unhold event.
		//
		// System::assert_has_event(
		// 	pallet_balances::Event::<Test>::Unlocked { who: validator.account_id, amount: 300 }
		// 		.into(),
		// );
	});
}

#[rstest]
fn not_bound(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let origin = origin(0);

		let res = Staking::self_unstake_currency(origin, 100);
		assert_noop!(res, Error::<Test>::NotBound);
	});
}

#[apply(dpos)]
fn contract_is_binding(
	mut ext: TestExternalities,
	permission: PermissionType,
	#[values(1, MinimumStakingPeriod::get().get() / 2, MinimumStakingPeriod::get().get() - 1)]
	session: u32,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let _ = EndowParams::default().account_id(validator.account_id).endow();

		Staking::self_stake_currency(validator.origin.clone(), MinimumStakingAmount::get())
			.expect("could stake currency");

		run_until::<AllPalletsWithSystem, Test>(ToSession(session));

		let res = Staking::self_unstake_currency(validator.origin, MinimumStakingAmount::get());
		assert_noop!(res, Error::<Test>::EarlyUnstake);
	});
}

#[apply(dpos)]
fn too_small_unstake(
	mut ext: TestExternalities,
	permission: PermissionType,
	#[values(0, 1, MinimumStakingAmount::get() - 1)] amount: Balance,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let _ = EndowParams::default().account_id(validator.account_id).endow();

		Staking::self_stake_currency(validator.origin.clone(), MinimumStakingAmount::get())
			.expect("could stake currency");

		skip_min_staking_period();

		let res = Staking::self_unstake_currency(validator.origin, amount);
		assert_noop!(res, Error::<Test>::TooSmallUnstake);
	});
}

#[apply(dpos)]
fn not_enough_currency_stake(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let delegation_details = EndowParams::default().account_id(validator.account_id).endow();

		Staking::self_stake_currency(validator.origin.clone(), MinimumStakingAmount::get())
			.expect("could stake currency");

		Staking::self_stake_nft(validator.origin.clone(), delegation_details.delegator_nft)
			.expect("could delegate nft");

		skip_min_staking_period();

		let res = Staking::self_unstake_currency(validator.origin, MinimumStakingAmount::get() + 1);
		assert_noop!(res, Error::<Test>::InsufficientFunds);
	});
}

#[apply(dpos)]
fn unstake_would_dust_stake(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let _ = EndowParams::default().account_id(validator.account_id).endow();

		Staking::self_stake_currency(validator.origin.clone(), MinimumStakingAmount::get() + 1)
			.expect("could stake currency");

		skip_min_staking_period();

		let res = Staking::self_unstake_currency(validator.origin, MinimumStakingAmount::get());
		assert_noop!(res, Error::<Test>::WouldDust);
	});
}

#[apply(dpos)]
fn can_unstake_all(
	mut ext: TestExternalities,
	permission: PermissionType,
	#[values(MinimumStakingAmount::get(), 10_000)] amount: Balance,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let _ = EndowParams::default().account_id(validator.account_id).endow();

		Staking::self_stake_currency(validator.origin.clone(), amount)
			.expect("could stake currency");

		skip_min_staking_period();

		let res = Staking::self_unstake_currency(validator.origin, amount);
		assert_ok!(res, ());
	});
}

#[apply(dpos)]
fn can_unstake_dusted_by_slash(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let _ = EndowParams::default().account_id(validator.account_id).endow();

		Staking::self_stake_currency(validator.origin.clone(), MinimumStakingAmount::get())
			.expect("could stake currency");

		let mut contract =
			Pallet::<Test>::current_contract(&validator.account_id, &validator.account_id).unwrap();
		contract.stake.currency = 2;
		Pallet::<Test>::stage_contract(&validator.account_id, &validator.account_id, contract);

		skip_min_staking_period();

		let res = Staking::self_unstake_currency(validator.origin, 2);
		assert_ok!(res, ());
	});
}
