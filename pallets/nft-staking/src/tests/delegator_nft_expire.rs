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

		run_until::<AllPalletsWithoutSystem, _>(ToSession::current_plus(expiry));

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

		System::assert_has_event(
			Event::<Test>::NftUndelegated {
				validator: validator.account_id,
				staker: delegator.account_id,
				item_id: delegator.delegator_nft,
			}
			.into(),
		);

		next_session();

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
		run_until::<AllPalletsWithoutSystem, _>(ToSession::current_plus(expiry - 1));

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
