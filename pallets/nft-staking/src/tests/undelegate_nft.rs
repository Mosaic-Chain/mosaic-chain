use super::*;

fn undelegate_maybe_force(
	validator: AccountId,
	delegator: AccountId,
	item_id: u32,
	force: bool,
) -> DispatchResult {
	if force {
		Staking::force_undelegate_nft(RuntimeOrigin::root(), validator, delegator, item_id)
	} else {
		Staking::undelegate_nft(origin(delegator), item_id, validator)
	}
}

#[apply(force_cases)]
fn undelegate_nft_is_successful(mut ext: TestExternalities, force: bool) {
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

		let res = undelegate_maybe_force(
			validator.account_id,
			delegator.account_id,
			delegator.delegator_nft,
			force,
		);

		assert_ok!(res, ());

		assert_current_validator_stake!(
			&validator.account_id,
			Some(TotalValidatorStake { total_stake, .. }) if total_stake == NOMINAL_VALUE
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

		assert!(Pallet::<Test>::current_contract(&validator.account_id, &delegator.account_id)
			.is_none());
		assert!(!NftDelegationHandler::is_bound(&delegator.delegator_nft));
	});
}

#[apply(force_cases)]
fn can_undelegate_if_target_is_slacking(mut ext: TestExternalities, force: bool) {
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

		// is_slacking = current_index - chill_index > SlackingPeriod
		// => slacking since session `current` + `SlackingPeriod` + 1
		run_until::<AllPalletsWithSystem, Test>(ToSession::current_plus::<Test>(
			SlackingPeriod::get() + 1,
		));

		let res = undelegate_maybe_force(
			validator.account_id,
			delegator.account_id,
			delegator.delegator_nft,
			force,
		);

		assert_ok!(res, ());
	});
}

#[apply(force_cases)]
fn target_not_bound(mut ext: TestExternalities, force: bool) {
	ext.execute_with(|| {
		let validator = 0;
		let delegator = EndowParams::default().endow();

		let res =
			undelegate_maybe_force(validator, delegator.account_id, delegator.delegator_nft, force);
		assert_noop!(res, Error::<Test>::NotBound);
	});
}

#[apply(force_cases)]
fn target_not_dpos(mut ext: TestExternalities, #[case] force: bool) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::PoS).mint().bind();
		let delegator = EndowParams::default().endow();

		let res = undelegate_maybe_force(
			validator.account_id,
			delegator.account_id,
			delegator.delegator_nft,
			force,
		);

		assert_noop!(res, Error::<Test>::TargetNotDPoS);
	});
}

#[apply(force_cases)]
fn target_is_caller(mut ext: TestExternalities, force: bool) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().account_id(validator.account_id).endow();

		Staking::self_stake_nft(delegator.origin.clone(), delegator.delegator_nft)
			.expect("could delegate nft");

		if force {
			let res = Staking::undelegate_nft(
				validator.origin,
				delegator.delegator_nft,
				validator.account_id,
			);
			assert_noop!(res, Error::<Test>::InvalidTarget);
		} else {
			Staking::force_undelegate_nft(
				RuntimeOrigin::root(),
				validator.account_id,
				delegator.account_id,
				delegator.delegator_nft,
			)
			.expect("could forcefully undelegate nft");

			System::assert_last_event(
				Event::<Test>::NftUndelegated {
					validator: validator.account_id,
					staker: delegator.account_id,
					item_id: delegator.delegator_nft,
				}
				.into(),
			);
		}
	});
}

#[apply(force_cases)]
fn item_does_not_exists(mut ext: TestExternalities, force: bool) {
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

		let res = undelegate_maybe_force(
			validator.account_id,
			delegator.account_id,
			delegator.delegator_nft,
			force,
		);

		assert_noop!(res, Error::<Test>::NftNotBound); // TODO: should we have more specific errors?
	});
}

#[apply(force_cases)]
fn wrong_owner(mut ext: TestExternalities, force: bool) {
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

		let res = undelegate_maybe_force(
			validator.account_id,
			delegator.account_id,
			delegator.delegator_nft,
			force,
		);

		assert_noop!(res, Error::<Test>::NftNotBound); // TODO: should we have more specific errors?
	});
}

#[apply(force_cases)]
fn item_not_bound(mut ext: TestExternalities, force: bool) {
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

		let res = undelegate_maybe_force(
			validator.account_id,
			delegator.account_id,
			delegator.delegator_nft,
			force,
		);
		assert_noop!(res, Error::<Test>::NftNotBound);
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

		Staking::delegate_nft(
			delegator.origin.clone(),
			delegator.delegator_nft,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate nft");

		run_until::<AllPalletsWithSystem, Test>(ToSession(session));

		if force {
			Staking::force_undelegate_nft(
				RuntimeOrigin::root(),
				validator.account_id,
				delegator.account_id,
				delegator.delegator_nft,
			)
			.expect("could forcefully undelegate nft");

			System::assert_last_event(
				Event::<Test>::NftUndelegated {
					validator: validator.account_id,
					staker: delegator.account_id,
					item_id: delegator.delegator_nft,
				}
				.into(),
			);
		} else {
			let res = Staking::undelegate_nft(
				delegator.origin,
				delegator.delegator_nft,
				validator.account_id,
			);
			assert_noop!(res, Error::<Test>::EarlyUnstake);
		}
	});
}
