use super::*;

#[apply(permission_cases)]
fn unbind_is_successful(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();

		skip_min_staking_period();
		// Since on unbinding the contracts are terminated immediately, we need to skip an extra session to enable for that.
		next_session();

		Staking::chill_validator(validator.origin.clone()).expect("could chill validator");

		let res = Staking::unbind_validator(validator.origin.clone());
		assert_ok!(res, ());

		assert!(Validators::<Test>::get(validator.account_id).is_none());
		assert!(ValidatorStates::<Test>::get(validator.account_id).is_none());
		assert!(!TotalValidatorStakes::<Test>::get(validator.account_id).exists());
		assert!(!Contracts::<Test>::get(validator.account_id, validator.account_id).exists());
		assert!(!NftStakingHandler::get().bound_tokens.contains_key(&validator.account_id));

		System::assert_last_event(Event::<Test>::ValidatorUnbound(validator.account_id).into());
	});
}

#[rstest]
fn not_binding_contracts_kicked(mut ext: TestExternalities) {
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

		let res = Staking::unbind_validator(validator.origin);
		assert_ok!(res, ());

		assert!(!TotalValidatorStakes::<Test>::get(validator.account_id).exists());
		assert!(!Contracts::<Test>::get(validator.account_id, validator.account_id).exists());
		assert!(!Contracts::<Test>::get(validator.account_id, delegator.account_id).exists());
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

#[rstest]
fn not_bound(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let origin = origin(0);

		let res = Staking::unbind_validator(origin);
		assert_noop!(res, Error::<Test>::NotBound);
	});
}

#[apply(permission_cases)]
fn not_chilled(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();

		skip_min_staking_period();
		next_session();

		let res = Staking::unbind_validator(validator.origin);
		assert_noop!(res, Error::<Test>::CallerIsNotChilled);
	});
}

#[apply(permission_cases)]
fn self_contract_is_binding(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();

		let midpoint = u32::from(MinimumStakingPeriod::get()) / 2;
		run_until::<AllPalletsWithoutSystem, _>(ToSession(midpoint));

		Staking::chill_validator(validator.origin.clone()).expect("could chill validator");

		let res = Staking::unbind_validator(validator.origin);
		assert_noop!(res, Error::<Test>::BindingContractExists);
	});
}

#[rstest]
fn binding_contract_exists(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();

		skip_min_staking_period();
		next_session();

		let delegator = EndowParams::default().endow();

		Staking::delegate_currency(
			delegator.origin,
			100,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			Perbill::from_percent(1),
		)
		.expect("could delegate currency");

		Staking::chill_validator(validator.origin.clone()).expect("could chill validator");

		let res = Staking::unbind_validator(validator.origin);
		assert_noop!(res, Error::<Test>::BindingContractExists);
	});
}

#[apply(permission_cases)]
fn drafted(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();

		ValidatorSet::put(Vec::from([validator.account_id]));

		// Make sure alice is selected and self-contract is expired
		skip_min_staking_period();
		next_session();

		Staking::chill_validator(validator.origin.clone()).expect("could chill validator");

		let res = Staking::unbind_validator(validator.origin);
		assert_err!(res, Error::<Test>::ValidatorAlreadySelected);
	});
}
