use super::*;

#[rstest]
fn undelegate_nft_is_successful(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		Staking::delegate_nft(
			delegator.origin.clone(),
			delegator.delegator_nft,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate nft");

		skip_min_staking_period();

		let res = Staking::undelegate_nft(
			delegator.origin,
			delegator.delegator_nft,
			validator.account_id,
		);
		assert_ok!(res, ());

		assert_current_validator_stake!(
			&validator.account_id,
			Some(TotalValidatorStake { total_stake, .. }) if *total_stake == NOMINAL_VALUE
		);

		assert_current_contract!(&validator.account_id, &delegator.account_id,
			Some(Contract {
				stake: Stake {
					delegated_nfts,
					..
				},
				..
			}) if delegated_nfts.is_empty());

		System::assert_last_event(
			Event::<Test>::NftUndelegated {
				validator: validator.account_id,
				staker: delegator.account_id,
				item_id: delegator.delegator_nft,
			}
			.into(),
		);

		next_session();

		assert!(!Contracts::<Test>::get(validator.account_id, delegator.account_id).exists());
		assert!(!NftDelegationHandler::is_bound(&delegator.delegator_nft));
	});
}

#[rstest]
fn can_undelegate_if_target_is_slacking(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		Staking::delegate_nft(
			delegator.origin.clone(),
			delegator.delegator_nft,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate nft");

		ValidatorStates::<Test>::mutate_extant(validator.account_id, |s| {
			*s = ValidatorState::Chilled(Session::current_index());
		});

		// 1 block = 1 session
		// (n + 1)th block = nth session
		// is_slacking = current_index - chill_index > SlackingPeriod
		// => chill_imdex is 0
		// => slacking since session SlackingPeriod + 1
		// => slacking since block SlackingPeriod + 2
		run_to_block(u64::from(SlackingPeriod::get()) + 2, |_| {});

		let res = Staking::undelegate_nft(
			delegator.origin,
			delegator.delegator_nft,
			validator.account_id,
		);
		assert_ok!(res, ());
	});
}

#[rstest]
fn target_not_bound(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = 0;
		let delegator = EndowParams::default().endow();

		let res = Staking::undelegate_nft(delegator.origin, delegator.delegator_nft, validator);
		assert_noop!(res, Error::<Test>::NotBound);
	});
}

#[rstest]
fn target_not_dpos(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::PoS).mint().bind();
		let delegator = EndowParams::default().endow();

		let res = Staking::undelegate_nft(
			delegator.origin,
			delegator.delegator_nft,
			validator.account_id,
		);
		assert_noop!(res, Error::<Test>::TargetNotDPoS);
	});
}

#[rstest]
fn target_is_caller(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().account_id(validator.account_id).endow();

		let res = Staking::undelegate_nft(
			validator.origin,
			delegator.delegator_nft,
			validator.account_id,
		);
		assert_noop!(res, Error::<Test>::InvalidTarget);
	});
}

#[rstest]
fn item_does_not_exists(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		// ensure validator.account_id and delegator.account_id have a contract
		Staking::delegate_currency(
			delegator.origin.clone(),
			MinimumStakingAmount::get(),
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate currency");

		let res = Staking::undelegate_nft(delegator.origin, 42, validator.account_id);
		assert_noop!(res, Error::<Test>::NftNotBound); // TODO: should we have more specific errors?
	});
}

#[rstest]
fn wrong_owner(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		// ensure validator and delegator have a contract
		Staking::delegate_currency(
			delegator.origin.clone(),
			MinimumStakingAmount::get(),
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate currency");

		let res = Staking::undelegate_nft(
			delegator.origin,
			delegator.delegator_nft,
			validator.account_id,
		);
		assert_noop!(res, Error::<Test>::NftNotBound); // TODO: should we have more specific errors?
	});
}

#[rstest]
fn item_not_bound(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		// ensure validator.account_id and delegator.account_id have a contract
		Staking::delegate_currency(
			delegator.origin.clone(),
			MinimumStakingAmount::get(),
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate currency");

		let res = Staking::undelegate_nft(
			delegator.origin,
			delegator.delegator_nft,
			validator.account_id,
		);
		assert_noop!(res, Error::<Test>::NftNotBound);
	});
}

#[rstest]
fn binding_contract(
	mut ext: TestExternalities,
	#[values(2, u32::from(MinimumStakingPeriod::get()) / 2, u32::from(MinimumStakingPeriod::get()))]
	block: u32,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		Staking::delegate_nft(
			delegator.origin.clone(),
			delegator.delegator_nft,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate nft");

		run_to_block(block.into(), |_| {});

		let res = Staking::undelegate_nft(
			delegator.origin,
			delegator.delegator_nft,
			validator.account_id,
		);
		assert_noop!(res, Error::<Test>::EarlyUnstake);
	});
}
