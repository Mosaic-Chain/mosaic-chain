use super::*;

fn kick_maybe_force(validator: AccountId, delegator: AccountId, force: bool) -> DispatchResult {
	if force {
		Staking::force_kick(RuntimeOrigin::root(), validator, delegator)
	} else {
		Staking::kick(origin(validator), delegator)
	}
}

#[apply(force_cases)]
fn kick_is_successful(mut ext: TestExternalities, force: bool) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		Staking::delegate_currency(
			delegator.origin.clone(),
			100,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate currency");

		Staking::delegate_nft(
			delegator.origin,
			delegator.delegator_nft,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate nft");

		skip_min_staking_period();

		let res = kick_maybe_force(validator.account_id, delegator.account_id, force);
		assert_ok!(res, ());

		let validator_stake = Staking::current_total_validator_stake(&validator.account_id)
			.expect("validator has stake");

		// total_stake is modified, but the contract is not yet relinquished
		assert!(
			validator_stake.total_stake
				== NftStakingHandler::nominal_value(&validator.permission_nft)
					.expect("nominal value is present")
		);

		assert!(validator_stake.contract_count == 2);
		assert!(Pallet::<Test>::current_contract(&validator.account_id, &delegator.account_id)
			.is_some_and(|c| c.stake.is_empty()));

		System::assert_has_event(
			Event::StakerKicked {
				validator: validator.account_id,
				staker: delegator.account_id,
				reason: KickReason::Manual,
			}
			.into(),
		);

		// delegator nft is still bound
		assert!(NftDelegationHandlerStore::get()
			.bound_tokens
			.contains_key(&delegator.delegator_nft));

		next_session();

		assert!(Pallet::<Test>::current_contract(&validator.account_id, &delegator.account_id)
			.is_none()); // now the contract is removed

		assert_current_validator_stake!(
			&validator.account_id,
			Some(TotalValidatorStake { contract_count: 1, .. })
		);

		assert!(!NftDelegationHandlerStore::get()
			.bound_tokens
			.contains_key(&delegator.delegator_nft));

		// pallet_balances does not have an Unhold event.
		//
		// System::assert_has_event(
		// 	pallet_balances::Event::Unlocked { who: delegator.account_id.clone(), amount: 100 }
		// 		.into(),
		// );
	});
}

#[apply(force_cases)]
fn not_bound(mut ext: TestExternalities, force: bool) {
	ext.execute_with(|| {
		let res = kick_maybe_force(0, 1, force);
		assert_noop!(res, Error::<Test>::NotBound);
	});
}

#[apply(force_cases)]
fn chilled(mut ext: TestExternalities, force: bool) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		Staking::delegate_currency(
			delegator.origin,
			100,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate currency");

		skip_min_staking_period();

		Staking::chill_validator(validator.origin.clone()).expect("could chill validator");

		if force {
			Staking::force_kick(RuntimeOrigin::root(), validator.account_id, delegator.account_id)
				.expect("could kick with force");

			System::assert_has_event(
				Event::StakerKicked {
					validator: validator.account_id,
					staker: delegator.account_id,
					reason: KickReason::Manual,
				}
				.into(),
			);
		} else {
			let res = Staking::kick(validator.origin, delegator.account_id);
			assert_noop!(res, Error::<Test>::CallerIsChilled);
		}
	});
}

#[apply(force_cases)]
fn not_dpos(mut ext: TestExternalities, force: bool) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::PoS).mint().bind();

		let res = kick_maybe_force(validator.account_id, 1, force);
		assert_noop!(res, Error::<Test>::CallerNotDPoS);
	});
}

#[apply(force_cases)]
fn caller_is_target(mut ext: TestExternalities, force: bool) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let res = kick_maybe_force(validator.account_id, validator.account_id, force);
		assert_noop!(res, Error::<Test>::InvalidCaller);
	});
}

#[apply(force_cases)]
fn no_contract(mut ext: TestExternalities, force: bool) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let res = kick_maybe_force(validator.account_id, 1, force);
		assert_noop!(res, Error::<Test>::NoContract);
	});
}

#[apply(force_cases)]
fn binding_contract(
	mut ext: TestExternalities,
	#[values(1, MinimumStakingPeriod::get().get() / 2, MinimumStakingPeriod::get().get() - 1)]
	session: u32,
	force: bool,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		Staking::delegate_currency(
			delegator.origin,
			100,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate currency");
		run_until::<AllPalletsWithSystem, Test>(ToSession(session));

		if force {
			Staking::force_kick(RuntimeOrigin::root(), validator.account_id, delegator.account_id)
				.expect("could kick with force");

			System::assert_has_event(
				Event::StakerKicked {
					validator: validator.account_id,
					staker: delegator.account_id,
					reason: KickReason::Manual,
				}
				.into(),
			);
		} else {
			let res = Staking::kick(validator.origin, delegator.account_id);
			assert_noop!(res, Error::<Test>::EarlyKick);
		}
	});
}
