use super::*;

fn unbind_maybe_force(validator: AccountId, contract_count: u32, force: bool) -> DispatchResult {
	if force {
		Staking::force_unbind_validator(RuntimeOrigin::root(), validator, contract_count)
	} else {
		Staking::unbind_validator(origin(validator), contract_count)
	}
}

#[apply(permission_x_force_cases)]
fn unbind_is_successful(mut ext: TestExternalities, permission: PermissionType, force: bool) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();

		skip_min_staking_period();
		// Since on unbinding the contracts are terminated immediately, we need to skip an extra session to enable for that.
		next_session();

		Staking::chill_validator(validator.origin.clone()).expect("could chill validator");

		let res = unbind_maybe_force(validator.account_id, 1, force);
		assert_ok!(res, ());

		assert!(Validators::<Test>::get(validator.account_id).is_none());
		assert!(ValidatorStates::<Test>::get(validator.account_id).is_none());
		assert!(Staking::current_total_validator_stake(&validator.account_id).is_none());
		assert!(Pallet::<Test>::current_contract(&validator.account_id, &validator.account_id)
			.is_none());
		assert!(!NftStakingHandler::get().bound_tokens.contains_key(&validator.account_id));

		System::assert_last_event(Event::<Test>::ValidatorUnbound(validator.account_id).into());
	});
}

#[apply(force_cases)]
fn not_binding_contracts_kicked(mut ext: TestExternalities, force: bool) {
	ext.execute_with(|| {
		let validator = BindParams::default().mint().bind();
		let delegator = EndowParams::default().endow();

		Staking::delegate_currency(
			delegator.origin.clone(),
			100,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			Perbill::from_percent(1),
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

		// All contract's minimum period expires here
		skip_min_staking_period();
		next_session();

		Staking::chill_validator(validator.origin.clone()).expect("could chill validator");

		let res = unbind_maybe_force(validator.account_id, 2, force);
		assert_ok!(res, ());

		assert!(Staking::current_total_validator_stake(&validator.account_id).is_none());
		assert!(Pallet::<Test>::current_contract(&validator.account_id, &validator.account_id)
			.is_none());
		assert!(Pallet::<Test>::current_contract(&validator.account_id, &delegator.account_id)
			.is_none());
		assert!(!NftDelegationHandlerStore::get()
			.bound_tokens
			.contains_key(&delegator.delegator_nft));

		System::assert_has_event(
			Event::<Test>::StakerKicked {
				validator: validator.account_id,
				staker: delegator.account_id,
				reason: KickReason::Unbind,
			}
			.into(),
		);

		System::assert_has_event(
			Event::<Test>::StakerKicked {
				validator: validator.account_id,
				staker: validator.account_id,
				reason: KickReason::Unbind,
			}
			.into(),
		);
	});
}

#[apply(force_cases)]
fn not_bound(mut ext: TestExternalities, force: bool) {
	ext.execute_with(|| {
		let res = unbind_maybe_force(0, 1, force);
		assert_noop!(res, Error::<Test>::NotBound);
	});
}

#[apply(permission_x_force_cases)]
fn not_chilled(mut ext: TestExternalities, permission: PermissionType, force: bool) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();

		skip_min_staking_period();
		next_session();

		if force {
			Staking::force_unbind_validator(RuntimeOrigin::root(), validator.account_id, 1)
				.expect("could unbind unchilled validator forcefully");
			System::assert_last_event(Event::<Test>::ValidatorUnbound(validator.account_id).into());
		} else {
			let res = Staking::unbind_validator(validator.origin, 1);
			assert_noop!(res, Error::<Test>::CallerIsNotChilled);
		}
	});
}

#[apply(permission_x_force_cases)]
fn self_contract_is_binding(mut ext: TestExternalities, permission: PermissionType, force: bool) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();

		let midpoint = u32::from(MinimumStakingPeriod::get()) / 2;
		run_until::<AllPalletsWithSystem, Test>(ToSession(midpoint));

		Staking::chill_validator(validator.origin.clone()).expect("could chill validator");

		if force {
			Staking::force_unbind_validator(RuntimeOrigin::root(), validator.account_id, 1)
				.expect("could forcefully unbind validator with binding self-contract");
			System::assert_last_event(Event::<Test>::ValidatorUnbound(validator.account_id).into());
		} else {
			let res = Staking::unbind_validator(validator.origin, 1);
			assert_noop!(res, Error::<Test>::BindingContractExists);
		}
	});
}

#[apply(force_cases)]
fn binding_contract_exists(mut ext: TestExternalities, force: bool) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();

		skip_min_staking_period();

		let delegator = EndowParams::default().endow();

		Staking::delegate_currency(
			delegator.origin,
			100,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			Perbill::from_percent(1),
		)
		.expect("could delegate currency");

		next_session();

		Staking::chill_validator(validator.origin.clone()).expect("could chill validator");

		if force {
			Staking::force_unbind_validator(RuntimeOrigin::root(), validator.account_id, 2)
				.expect("could forcefully unbind validator with binding contract");
			System::assert_last_event(Event::<Test>::ValidatorUnbound(validator.account_id).into());
		} else {
			let res = Staking::unbind_validator(validator.origin, 2);
			assert_noop!(res, Error::<Test>::BindingContractExists);
		}
	});
}

#[apply(force_cases)]
fn uncommitted_state(mut ext: TestExternalities, force: bool) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();

		skip_min_staking_period();
		next_session();

		// Re-stage committed value
		let current = Staking::current_total_validator_stake(&validator.account_id).unwrap();
		Staking::stage_total_validator_stake(&validator.account_id, current);

		Staking::chill_validator(validator.origin.clone()).expect("could chill validator");

		let res = unbind_maybe_force(validator.account_id, 2, force);
		assert_noop!(res, Error::<Test>::UncommittedState);
	});
}

#[apply(permission_x_force_cases)]
fn drafted(mut ext: TestExternalities, permission: PermissionType, force: bool) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();

		ValidatorSet::put(Vec::from([validator.account_id]));

		// Make sure alice is selected and self-contract is expired
		skip_min_staking_period();
		next_session();

		Staking::chill_validator(validator.origin.clone()).expect("could chill validator");

		let res = unbind_maybe_force(validator.account_id, 1, force);
		assert_err!(res, Error::<Test>::ValidatorAlreadySelected);
	});
}
