use super::*;

#[rstest]
fn slashing_works(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().nft_nominal_value(100).mint().bind();
		let delegator = EndowParams::default().currency(5000).nft_nominal_value(100).endow();

		Staking::delegate_currency(
			delegator.origin.clone(),
			100,
			validator.account_id.clone(),
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate currency");

		Staking::delegate_nft(
			delegator.origin,
			delegator.delegator_nft,
			validator.account_id.clone(),
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate nft");

		// Wait a session for the validator to become slashable and delegation to become committed
		next_session();

		validator.offend();
		next_session();

		System::assert_has_event(
			Event::<Test>::Slash {
				offender: validator.account_id.clone(),
				staker: validator.account_id.clone(),
				currency: 0,
				delegator_nfts: vec![],
				permission_nft: Some(1), // 1% of 100
			}
			.into(),
		);

		System::assert_has_event(
			Event::<Test>::Slash {
				offender: validator.account_id.clone(),
				staker: delegator.account_id.clone(),
				currency: 1,
				delegator_nfts: vec![(delegator.delegator_nft, 1)],
				permission_nft: None,
			}
			.into(),
		);

		assert_current_validator_stake!(
			&validator.account_id,
			Some(&TotalValidatorStake { contract_count: 2, total_stake: 297 })
		);

		assert_current_contract!(
			&validator.account_id,
			&validator.account_id,
			Some(&Contract { stake: Stake { currency: 0, permission_nft: Some(99), .. }, .. })
		);

		assert_current_contract!(
			&validator.account_id,
			&delegator.account_id,
			Some(&Contract {
				stake: Stake {
					currency: 99,
					ref delegated_nfts,
					..
				},
				..
			}) if delegated_nfts.to_vec() == [(delegator.delegator_nft, 99)]
		);

		assert_ok!(NftStakingHandler::nominal_value(&validator.permission_nft), 99);
		assert_ok!(NftDelegationHandler::nominal_value(&delegator.delegator_nft), 99);
		assert_eq!(Balances::total_balance(&delegator.account_id), 4999);
	});
}

#[rstest]
fn staged_assets_are_not_slashed(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().nft_nominal_value(100).mint().bind();
		let delegator = EndowParams::default().currency(5000).nft_nominal_value(100).endow();

		Staking::delegate_currency(
			delegator.origin.clone(),
			100,
			validator.account_id.clone(),
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate currency");

		// Wait a session for the validator to become slashable and some delegation to become committed
		next_session();

		// This portion will not yet be slashed, but will bring the amount to 200 after first batch is slashed
		Staking::delegate_currency(
			delegator.origin.clone(),
			101,
			validator.account_id.clone(),
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate currency");

		Staking::delegate_nft(
			delegator.origin,
			delegator.delegator_nft,
			validator.account_id.clone(),
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate nft");

		validator.offend();
		next_session();

		System::assert_has_event(
			Event::<Test>::Slash {
				offender: validator.account_id.clone(),
				staker: delegator.account_id.clone(),
				currency: 1,            // 1% of the first batch
				delegator_nfts: vec![], // the nft is not slashed yet as its not commited
				permission_nft: None,
			}
			.into(),
		);

		validator.offend();
		next_session();

		System::assert_has_event(
			Event::<Test>::Slash {
				offender: validator.account_id.clone(),
				staker: delegator.account_id.clone(),
				currency: 2,                                        // 1% of the total currency stake
				delegator_nfts: vec![(delegator.delegator_nft, 1)], // the nft is now slashed
				permission_nft: None,
			}
			.into(),
		);
	});
}
