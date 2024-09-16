use super::*;

#[rstest]
fn delegate_nft_is_successful(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		let res = Staking::delegate_nft(
			delegator.origin,
			delegator.delegator_nft,
			validator.account_id.clone(),
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);

		assert_ok!(res, ());

		System::assert_last_event(
			Event::NftDelegated {
				validator: validator.account_id.clone(),
				staker: delegator.account_id.clone(),
				item_id: delegator.delegator_nft,
			}
			.into(),
		);

		assert!(NftDelegationHandler::is_bound(&delegator.delegator_nft));

		assert_current_validator_stake!(
			&validator.account_id,
			Some(TotalValidatorStake { total_stake, .. }) if *total_stake == NOMINAL_VALUE + NOMINAL_VALUE
		);

		assert_current_contract!(&validator.account_id, &delegator.account_id,
			Some(Contract {
				stake: Stake {
					delegated_nfts,
					permission_nft: None,
					..
				},
				..
			}) if delegated_nfts[0] == (delegator.delegator_nft, NOMINAL_VALUE));
	});
}

#[rstest]
fn staking_period_resets(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		run_to_block(50, |_| {});

		Staking::delegate_nft(
			delegator.origin,
			delegator.delegator_nft,
			validator.account_id.clone(),
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate nft");

		let new_period_end = u32::from(MinimumStakingPeriod::get()) + Session::current_index();

		assert_current_contract!(&validator.account_id, &delegator.account_id,
			Some(Contract { min_staking_period_end, .. } ) if *min_staking_period_end == new_period_end);
	});
}

#[rstest]
fn new_commission_applies(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		// Delegate some currency first to create contract.
		Staking::delegate_currency(
			delegator.origin.clone(),
			MinimumStakingAmount::get(),
			validator.account_id.clone(),
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate nft");

		assert_current_contract!(&validator.account_id, &delegator.account_id,
			Some(Contract { commission, .. } ) if *commission == MinimumCommission::get());

		let new_commission = Perbill::from_percent(42);

		Staking::set_commission(validator.origin, new_commission).expect("could set commission");

		Staking::delegate_nft(
			delegator.origin,
			delegator.delegator_nft,
			validator.account_id.clone(),
			MinimumStakingPeriod::get().into(),
			new_commission,
		)
		.expect("could delegate nft");

		assert_current_contract!(&validator.account_id, &delegator.account_id,
			Some(Contract { commission, .. } ) if *commission == new_commission);
	});
}

#[rstest]
fn target_not_bound(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = account(0);
		let delegator = EndowParams::default().endow();

		let res = Staking::delegate_nft(
			delegator.origin,
			delegator.delegator_nft,
			validator,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);

		assert_noop!(res, Error::<Test>::NotBound);
	});
}

#[rstest]
fn target_chilled(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		Staking::chill_validator(validator.origin).expect("could chill validator");

		let res = Staking::delegate_nft(
			delegator.origin,
			delegator.delegator_nft,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);

		assert_noop!(res, Error::<Test>::TargetIsChilled);
	});
}

#[rstest]
fn target_not_dpos(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::PoS).mint().bind();
		let delegator = EndowParams::default().endow();

		let res = Staking::delegate_nft(
			delegator.origin,
			delegator.delegator_nft,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);

		assert_noop!(res, Error::<Test>::TargetNotDPoS);
	});
}

#[rstest]
fn target_denies_delegations(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		Staking::disable_delegations(validator.origin).expect("could deny delegations");

		let res = Staking::delegate_nft(
			delegator.origin,
			delegator.delegator_nft,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);

		assert_noop!(res, Error::<Test>::TargetDeniesDelegations);
	});
}

#[rstest]
fn staking_period_slippage(
	mut ext: TestExternalities,
	#[values(u32::from(MinimumStakingPeriod::get()) - 1, u32::from(MinimumStakingPeriod::get()) + 1)]
	observed: u32,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		let res = Staking::delegate_nft(
			delegator.origin,
			delegator.delegator_nft,
			validator.account_id,
			observed,
			MinimumCommission::get(),
		);

		assert_noop!(res, Error::<Test>::SlippageExceeded);
	});
}

#[rstest]
fn commission_slippage(
	mut ext: TestExternalities,
	#[values(MinimumCommission::get().less_epsilon(), MinimumCommission::get().plus_epsilon())]
	observed: Perbill,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		let res = Staking::delegate_nft(
			delegator.origin,
			delegator.delegator_nft,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			observed,
		);

		assert_noop!(res, Error::<Test>::SlippageExceeded);
	});
}

#[rstest]
fn target_is_caller(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().account_id(validator.account_id.clone()).endow();

		let res = Staking::delegate_nft(
			validator.origin,
			delegator.delegator_nft,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get().plus_epsilon(),
		);

		assert_noop!(res, Error::<Test>::InvalidTarget);
	});
}

#[rstest]
fn item_does_not_exist(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		let res = Staking::delegate_nft(
			delegator.origin,
			42,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);
		assert_noop!(res, NftDeleationHandlerError::TokenDoesNotExist);
	});
}

#[rstest]
fn item_wrong_owner(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		let res = Staking::delegate_nft(
			origin(account(1)),
			delegator.delegator_nft,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);
		assert_noop!(res, NftDeleationHandlerError::WrongOwner);
	});
}

#[rstest]
fn item_already_bound(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		Staking::delegate_nft(
			delegator.origin.clone(),
			delegator.delegator_nft,
			validator.account_id.clone(),
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate nft");

		let res = Staking::delegate_nft(
			delegator.origin,
			delegator.delegator_nft,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);

		assert_noop!(res, NftDeleationHandlerError::AlreadyBound);
	});
}

#[rstest]
fn too_small_nominal_value(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		NftDelegationHandler::set_nominal_value(
			&delegator.delegator_nft,
			MinimumStakingAmount::get() - 1,
		)
		.expect("could set nominal value");

		let res = Staking::delegate_nft(
			delegator.origin.clone(),
			delegator.delegator_nft,
			validator.account_id.clone(),
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);

		assert_noop!(res, Error::<Test>::LowNominalValue);
	});
}

#[rstest]
fn item_expires_early(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();

		let expiry = u32::from(MinimumStakingPeriod::get()) - 1;
		let delegator = EndowParams::default().nft_expiry(expiry).endow();

		let res = Staking::delegate_nft(
			delegator.origin.clone(),
			delegator.delegator_nft,
			validator.account_id.clone(),
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);

		assert_noop!(res, Error::<Test>::NftExpiresEarly);
	});
}

#[rstest]
fn target_would_be_overdominant(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();

		let nominal_value =
			MaximumStakePercentage::get() * (Balances::total_issuance() - NOMINAL_VALUE);
		let delegator = EndowParams::default().nft_nominal_value(nominal_value).endow();

		let res = Staking::delegate_nft(
			delegator.origin,
			delegator.delegator_nft,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);

		assert_noop!(res, Error::<Test>::OverdominantStake);
	});
}
