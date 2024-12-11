use super::*;

#[apply(dpos)]
fn self_unstake_nft_is_successful(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let delegation_details = EndowParams::default().account_id(validator.account_id).endow();

		Staking::self_stake_nft(validator.origin.clone(), delegation_details.delegator_nft)
			.expect("could stake nft");

		skip_min_staking_period();

		let res = Staking::self_unstake_nft(validator.origin, delegation_details.delegator_nft);
		assert_ok!(res, ());

		assert_current_validator_stake!(
			&validator.account_id,
			Some(TotalValidatorStake { total_stake, .. }) if total_stake == NOMINAL_VALUE
		);

		assert_current_contract!(&validator.account_id, &validator.account_id,
			Some(Contract {
				stake: Stake {
					delegated_nfts,
					permission_nft: Some(NOMINAL_VALUE),
					..
				},
				..
			}) if delegated_nfts.is_empty());

		System::assert_last_event(
			Event::<Test>::NftUndelegated {
				validator: validator.account_id,
				staker: validator.account_id,
				item_id: delegation_details.delegator_nft,
			}
			.into(),
		);

		next_session();

		assert!(!NftDelegationHandler::is_bound(&delegation_details.delegator_nft));
	});
}

#[rstest]
fn caller_not_bound(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = 0;
		let delegation_details = EndowParams::default().account_id(validator).endow();

		let res = Staking::self_unstake_nft(origin(validator), delegation_details.delegator_nft);
		assert_noop!(res, Error::<Test>::NotBound);
	});
}

#[apply(dpos)]
fn item_does_not_exist(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();

		let res = Staking::self_unstake_nft(validator.origin, 42);
		assert_noop!(res, Error::<Test>::NftNotBound); // TODO: should we have more specific errors?
	});
}

#[rstest]
fn wrong_owner(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegation_details = EndowParams::default().endow();

		Staking::delegate_nft(
			delegation_details.origin,
			delegation_details.delegator_nft,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could bind nft");

		let res = Staking::self_unstake_nft(validator.origin, delegation_details.delegator_nft);
		assert_noop!(res, Error::<Test>::NftNotBound);
	});
}

#[apply(dpos)]
fn item_not_bound(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let delegation_details = EndowParams::default().account_id(validator.account_id).endow();

		let res = Staking::self_unstake_nft(validator.origin, delegation_details.delegator_nft);
		assert_noop!(res, Error::<Test>::NftNotBound);
	});
}

#[apply(dpos)]
fn self_contract_is_binding(
	mut ext: TestExternalities,
	permission: PermissionType,
	#[values(1, MinimumStakingPeriod::get().get() / 2, MinimumStakingPeriod::get().get() - 1)]
	session: u32,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let delegation_details = EndowParams::default().account_id(validator.account_id).endow();

		Staking::self_stake_nft(validator.origin.clone(), delegation_details.delegator_nft)
			.expect("could stake nft");

		run_until::<AllPalletsWithoutSystem, _>(ToSession(session));

		let res = Staking::self_unstake_nft(validator.origin, delegation_details.delegator_nft);
		assert_noop!(res, Error::<Test>::EarlyUnstake);
	});
}
