use super::*;

#[apply(permission_cases)]
fn topup_is_successful(
	mut ext: TestExternalities,
	permission: PermissionType,
	#[values(0, NOMINAL_VALUE/2, NOMINAL_VALUE - 1)] slashed_to: Balance,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let _ = EndowParams::default().account_id(validator.account_id.clone()).endow();

		// Simulate slash
		NftStakingHandler::set_nominal_value_of_bound(&validator.account_id, slashed_to)
			.expect("could set nominal value");
		TotalValidatorStakes::<Test>::mutate(&validator.account_id, |stake| {
			stake.ensure_staging_mut().map(|s| s.total_stake = slashed_to)
		});
		Contracts::<Test>::mutate(&validator.account_id, &validator.account_id, |contract| {
			contract.ensure_staging_mut().map(|c| c.stake.permission_nft = Some(slashed_to))
		});

		let res =
			Staking::topup(validator.origin, validator.permission_nft, NOMINAL_VALUE - slashed_to);
		assert_ok!(res, ());

		assert_ok!(NftStakingHandler::nominal_value(&validator.permission_nft), NOMINAL_VALUE);
		assert_ok!(
			NftStakingHandler::nominal_factor_of(&validator.account_id),
			Perbill::from_percent(100)
		);

		assert_current_validator_stake!(
			&validator.account_id,
			Some(&TotalValidatorStake { contract_count: 1, total_stake: NOMINAL_VALUE })
		);

		assert_current_contract!(
			&validator.account_id,
			&validator.account_id,
			Some(&Contract { stake: Stake { permission_nft: Some(NOMINAL_VALUE), .. }, .. })
		);

		System::assert_last_event(
			Event::PermissionNftTopup {
				validator: validator.account_id.clone(),
				item: validator.permission_nft,
				cost: NOMINAL_VALUE - slashed_to,
			}
			.into(),
		);

		System::assert_has_event(
			pallet_balances::Event::Withdraw {
				who: validator.account_id.clone(),
				amount: NOMINAL_VALUE - slashed_to,
			}
			.into(),
		);
	});
}

#[apply(permission_cases)]
fn topup_restores_normal_state_from_faulted(
	mut ext: TestExternalities,
	permission: PermissionType,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let _ = EndowParams::default().account_id(validator.account_id.clone()).endow();

		NftStakingHandler::set_nominal_value_of_bound(&validator.account_id, 10)
			.expect("could set nominal value");
		ValidatorStates::<Test>::mutate(&validator.account_id, |vstate| {
			*vstate = Some(ValidatorState::Faulted);
		});

		let res = Staking::topup(validator.origin, validator.permission_nft, 90);
		assert_ok!(res, ());

		assert_validator_state!(&validator.account_id, Some(ValidatorState::Normal));
	});
}

#[apply(permission_cases)]
fn topup_leaves_state_chilled(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let _ = EndowParams::default().account_id(validator.account_id.clone()).endow();

		Staking::chill_validator(validator.origin.clone()).expect("could chill validator");
		NftStakingHandler::set_nominal_value_of_bound(&validator.account_id, 10)
			.expect("could set nominal value");

		let res = Staking::topup(validator.origin, validator.permission_nft, 90);
		assert_ok!(res, ());

		assert_validator_state!(&validator.account_id, Some(ValidatorState::Chilled(0)));
	});
}

#[rstest]
fn item_does_not_exist(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let origin = origin(account(0));

		let res = Staking::topup(origin, 42, 100);
		assert_noop!(res, StakingHandlerError::TokenDoesNotExist);
	});
}

#[apply(permission_cases)]
fn wrong_owner(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let bob_origin = origin(account(1));

		let res = Staking::topup(bob_origin, validator.permission_nft, NOMINAL_VALUE);
		assert_noop!(res, Error::<Test>::TopupWrongOwner);
	});
}

#[apply(permission_cases)]
fn imbalance_grater_than_allowed(
	mut ext: TestExternalities,
	permission: PermissionType,
	#[values(0, NOMINAL_VALUE/2, NOMINAL_VALUE - 1)] slashed_to: Balance,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		NftStakingHandler::set_nominal_value_of_bound(&validator.account_id, slashed_to)
			.expect("could set nominal value");

		let res = Staking::topup(
			validator.origin,
			validator.permission_nft,
			NOMINAL_VALUE - slashed_to - 1,
		);
		assert_noop!(res, Error::<Test>::SlippageExceeded);
	});
}

#[apply(permission_cases)]
fn not_enough_free_balance(
	mut ext: TestExternalities,
	permission: PermissionType,
	#[values(0, NOMINAL_VALUE/2, NOMINAL_VALUE - 1)] slashed_to: Balance,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let _ = EndowParams::default()
			.account_id(validator.account_id.clone())
			.currency(NOMINAL_VALUE - slashed_to - 1)
			.endow();

		NftStakingHandler::set_nominal_value_of_bound(&validator.account_id, slashed_to)
			.expect("could set nominal value");

		let res =
			Staking::topup(validator.origin, validator.permission_nft, NOMINAL_VALUE - slashed_to);
		assert_noop!(res, sp_runtime::TokenError::FundsUnavailable);
	});
}

#[rstest]
fn fails_from_staked_balance(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().mint().bind();
		let _ = EndowParams::default()
			.account_id(validator.account_id.clone())
			.currency(NOMINAL_VALUE + 1) // +1 to not dust account
			.endow();

		let _avoid_overdominance = Balances::issue(10_000_000);

		Staking::self_stake_currency(validator.origin.clone(), NOMINAL_VALUE)
			.expect("could self stake currency");

		NftStakingHandler::set_nominal_value_of_bound(&validator.account_id, 0)
			.expect("could set nominal value");

		let res = Staking::topup(validator.origin, validator.permission_nft, NOMINAL_VALUE);
		assert_noop!(res, sp_runtime::TokenError::FundsUnavailable);
	});
}
