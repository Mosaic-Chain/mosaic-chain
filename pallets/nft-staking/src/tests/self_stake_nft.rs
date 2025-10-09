use super::*;

#[apply(dpos)]
fn self_stake_nft_is_successful(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let delegation_details = EndowParams::default().account_id(validator.account_id).endow();

		let res = Staking::self_stake_nft(validator.origin, delegation_details.delegator_nft);
		assert_ok!(res);

		assert_current_validator_stake!(
			&validator.account_id,
			Some(TotalValidatorStake { total_stake, .. }) if total_stake == NOMINAL_VALUE * 2
		);

		assert_current_contract!(&validator.account_id, &validator.account_id,
			Some(Contract {
				stake: Stake {
					delegated_nfts,
					permission_nft: Some(NOMINAL_VALUE),
					..
				},
				..
			}) if delegated_nfts[0] == (delegation_details.delegator_nft, NOMINAL_VALUE));

		assert!(NftDelegationHandlerStore::get()
			.bound_tokens
			.contains_key(&delegation_details.delegator_nft));

		assert_ok!(
			NftDelegationHandler::bind_metadata(&delegation_details.delegator_nft),
			validator.account_id
		);

		System::assert_last_event(
			Event::<Test>::NftDelegated {
				validator: validator.account_id,
				staker: validator.account_id,
				item_id: delegation_details.delegator_nft,
			}
			.into(),
		);
	});
}

#[apply(dpos)]
fn minimum_staking_period_resets(
	mut ext: TestExternalities,
	permission: PermissionType,
	#[values(1, MinimumStakingPeriod::get().get(), MinimumStakingPeriod::get().get(), 1000)]
	session: u32,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let delegation_details = EndowParams::default().account_id(validator.account_id).endow();

		run_until::<AllPalletsWithSystem, Test>(ToSession(session));

		let res = Staking::self_stake_nft(validator.origin, delegation_details.delegator_nft);
		assert_ok!(res, ());

		let new_period_end = u32::from(MinimumStakingPeriod::get()) + Session::current_index();

		assert_current_contract!(&validator.account_id, &validator.account_id,
			Some(Contract { min_staking_period_end, .. }) if min_staking_period_end == new_period_end
		);
	});
}

#[rstest]
fn caller_not_bound(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = 0;
		let origin = origin(validator);
		let delegation_details = EndowParams::default().account_id(validator).endow();

		let res = Staking::self_stake_nft(origin, delegation_details.delegator_nft);
		assert_noop!(res, Error::<Test>::NotBound);
	});
}

#[apply(permission_cases)]
fn chilled(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let delegation_details = EndowParams::default().account_id(validator.account_id).endow();

		Staking::chill_validator(validator.origin.clone()).expect("could chill validator");

		let res = Staking::self_stake_nft(validator.origin, delegation_details.delegator_nft);
		let expected = match permission {
			PermissionType::DPoS => Error::<Test>::TargetIsChilled,
			PermissionType::PoS => Error::<Test>::CallerNotDPoS,
		};
		assert_noop!(res, expected);
	});
}

#[apply(dpos)]
fn item_does_not_exist(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();

		let res = Staking::self_stake_nft(validator.origin, 42);
		assert_noop!(res, NftDelegationHandlerError::TokenDoesNotExist);
	});
}

#[apply(dpos)]
fn wrong_owner(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let delegation_details = EndowParams::default().endow();

		let res = Staking::self_stake_nft(validator.origin, delegation_details.delegator_nft);
		assert_noop!(res, NftDelegationHandlerError::WrongOwner);
	});
}

#[apply(dpos)]
fn too_small_nominal_value(
	mut ext: TestExternalities,
	permission: PermissionType,
	#[values(0, 1, MinimumStakingAmount::get() - 1)] amount: Balance,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let delegation_details = EndowParams::default()
			.nft_nominal_value(amount)
			.account_id(validator.account_id)
			.endow();

		let res = Staking::self_stake_nft(validator.origin, delegation_details.delegator_nft);
		assert_noop!(res, Error::<Test>::LowNominalValue);
	});
}

#[apply(dpos)]
fn item_expires_early(
	mut ext: TestExternalities,
	permission: PermissionType,
	#[values(0, 1, u32::from(MinimumStakingPeriod::get()) - 1)] expiry: SessionIndex,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let delegation_details = EndowParams::default()
			.nft_expiry(expiry)
			.account_id(validator.account_id)
			.endow();

		let res = Staking::self_stake_nft(validator.origin, delegation_details.delegator_nft);
		assert_err!(res, Error::<Test>::NftExpiresEarly);
	});
}

#[apply(dpos)]
fn item_already_bound(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let delegation_details = EndowParams::default().account_id(validator.account_id).endow();

		Staking::self_stake_nft(validator.origin.clone(), delegation_details.delegator_nft)
			.expect("could stake delegation_details.delegator_nft");

		let res = Staking::self_stake_nft(validator.origin, delegation_details.delegator_nft);
		assert_noop!(res, NftDelegationHandlerError::AlreadyBound);
	});
}

#[apply(dpos)]
fn would_be_overdominant(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let delegation_details = EndowParams::default().account_id(validator.account_id).endow();

		let nominal_value =
			MaximumStakePercentage::get() * (Balances::total_issuance() - NOMINAL_VALUE);

		NftDelegationHandler::set_nominal_value(&delegation_details.delegator_nft, nominal_value)
			.expect("could set nominal value");

		let res = Staking::self_stake_nft(validator.origin, delegation_details.delegator_nft);
		assert_err!(res, Error::<Test>::OverdominantStake);
	});
}
