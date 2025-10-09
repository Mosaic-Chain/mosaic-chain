use super::*;

#[rstest]
fn expiry_works(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();

		let expiry = MinimumStakingPeriod::get().get();
		let delegator = EndowParams::default().nft_expiry(expiry).endow();

		Staking::delegate_nft(
			delegator.origin,
			delegator.delegator_nft,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate nft");

		run_until::<AllPalletsWithSystem, Test>(ToSession::current_plus::<Test>(expiry));

		assert_current_validator_stake!(
			&validator.account_id,
			Some(TotalValidatorStake { total_stake, .. }) if total_stake == NOMINAL_VALUE
		);

		// Contract is cleaned up in `on_idle`
		assert!(Staking::current_contract(&validator.account_id, &delegator.account_id).is_none());

		System::assert_has_event(
			Event::<Test>::NftUndelegated {
				validator: validator.account_id,
				staker: delegator.account_id,
				item_id: delegator.delegator_nft,
			}
			.into(),
		);

		assert!(!NftDelegationHandler::is_bound(&delegator.delegator_nft));
	});
}

#[rstest]
fn expires_even_when_contract_is_binding(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();

		let expiry = MinimumStakingPeriod::get().get();
		let delegator = EndowParams::default().nft_expiry(expiry).endow();

		Staking::delegate_nft(
			delegator.origin.clone(),
			delegator.delegator_nft,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate nft");

		// Almost expires
		run_until::<AllPalletsWithSystem, Test>(ToSession::current_plus::<Test>(expiry - 1));

		// Update contract
		Staking::delegate_currency(
			delegator.origin,
			MinimumStakingAmount::get(),
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate currency");

		// Expires now
		next_session();

		System::assert_has_event(
			Event::<Test>::NftUndelegated {
				validator: validator.account_id,
				staker: delegator.account_id,
				item_id: delegator.delegator_nft,
			}
			.into(),
		);
	});
}
