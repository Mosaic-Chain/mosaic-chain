use crate::InverseSlashes;

use super::*;

#[rstest]
fn reward_is_distributed(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default()
			.permission(PermissionType::DPoS)
			.nft_nominal_value(100)
			.mint()
			.bind();

		// Avoid rewarding an empty account to make calculations easier
		let _endow_validator = EndowParams::default().account_id(validator.account_id).endow();
		let delegator = EndowParams::default().nft_nominal_value(50).endow();

		ValidatorSet::set(vec![validator.account_id]);
		SessionReward::set(1250); // 1250 * (1 - contribution) = 1000
		Staking::set_commission(validator.origin, Perbill::from_percent(10))
			.expect("could set commission rate");

		Staking::delegate_currency(
			delegator.origin.clone(),
			50,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			Perbill::from_percent(10),
		)
		.expect("could delegate currency");

		Staking::delegate_nft(
			delegator.origin.clone(),
			delegator.delegator_nft,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			Perbill::from_percent(10),
		)
		.expect("could delegate nft");

		next_session();
		next_session();

		System::assert_has_event(
			Event::<Test>::ContractReward {
				validator: validator.account_id,
				staker: delegator.account_id,
				reward: 450,
				commission: 50,
			}
			.into(),
		);

		System::assert_has_event(
			Event::<Test>::ContractReward {
				validator: validator.account_id,
				staker: validator.account_id,
				reward: 0,
				commission: 500,
			}
			.into(),
		);

		assert_current_validator_stake!(
			&validator.account_id,
			Some(TotalValidatorStake { total_stake: 1200, .. })
		);

		assert_current_contract!(
			&validator.account_id,
			&delegator.account_id,
			Some(Contract { stake: Stake { currency: 500, .. }, .. })
		);

		assert_current_contract!(
			&validator.account_id,
			&validator.account_id,
			Some(Contract { stake: Stake { currency: 550, permission_nft: Some(100), .. }, .. })
		);
	});
}

#[rstest]
fn does_not_reward_slashed(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();

		ValidatorSet::set(vec![validator.account_id]);
		SessionReward::set(1000);

		next_session();
		InverseSlashes::<Test>::insert(validator.account_id, Perbill::from_percent(100));
		next_session();

		assert_current_contract!(
			&validator.account_id,
			&validator.account_id,
			Some(Contract {
				stake: Stake { currency: 0, permission_nft: Some(NOMINAL_VALUE), .. },
				..
			})
		);
	});
}

#[rstest]
fn distribute_amongst_validators(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator1 = BindParams::default()
			.account_index(0)
			.permission(PermissionType::DPoS)
			.nft_nominal_value(100)
			.mint()
			.bind();

		let validator2 = BindParams::default()
			.account_index(1)
			.permission(PermissionType::DPoS)
			.nft_nominal_value(100)
			.mint()
			.bind();

		let delegator = EndowParams::default().nft_nominal_value(100).endow();

		ValidatorSet::set(vec![validator1.account_id, validator2.account_id]);

		SessionReward::set(1250);

		Staking::set_commission(validator1.origin, Perbill::from_percent(10))
			.expect("could set commissin rate");
		Staking::set_commission(validator2.origin, Perbill::from_percent(10))
			.expect("could set commissin rate");

		Staking::delegate_currency(
			delegator.origin.clone(),
			100,
			validator1.account_id,
			MinimumStakingPeriod::get().into(),
			Perbill::from_percent(10),
		)
		.expect("could delegate currency");

		Staking::delegate_nft(
			delegator.origin.clone(),
			delegator.delegator_nft,
			validator2.account_id,
			MinimumStakingPeriod::get().into(),
			Perbill::from_percent(10),
		)
		.expect("could delegate currency");

		next_session();
		next_session();

		System::assert_has_event(
			Event::<Test>::ContractReward {
				validator: validator1.account_id,
				staker: delegator.account_id,
				reward: 225,
				commission: 25,
			}
			.into(),
		);

		System::assert_has_event(
			Event::<Test>::ContractReward {
				validator: validator1.account_id,
				staker: validator1.account_id,
				reward: 0,
				commission: 250,
			}
			.into(),
		);

		System::assert_has_event(
			Event::<Test>::ContractReward {
				validator: validator2.account_id,
				staker: delegator.account_id,
				reward: 225,
				commission: 25,
			}
			.into(),
		);

		System::assert_has_event(
			Event::<Test>::ContractReward {
				validator: validator2.account_id,
				staker: validator2.account_id,
				reward: 0,
				commission: 250,
			}
			.into(),
		);
	});
}

#[rstest]
fn reward_empty_account(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().currency(0).endow();

		// Increase the total issuance by brute-force to avoid overdominant stake
		let _issuance = <Balances as frame_support::traits::Currency<AccountId>>::issue(10000);

		ValidatorSet::set(vec![validator.account_id]);
		SessionReward::set(1250); // 1250 * (1 - contribution) = 1000
		Staking::set_commission(validator.origin, Perbill::from_percent(10))
			.expect("could set commission rate");

		Staking::delegate_nft(
			delegator.origin.clone(),
			delegator.delegator_nft,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			Perbill::from_percent(10),
		)
		.expect("could delegate nft");

		next_session();
		next_session();

		let ed: Balance = <Test as pallet_balances::Config>::ExistentialDeposit::get();

		assert_eq!(Balances::total_balance(&validator.account_id), 550);
		assert_eq!(Balances::total_balance(&delegator.account_id), 450);

		assert_eq!(Balances::free_balance(validator.account_id), ed);
		assert_eq!(Balances::free_balance(delegator.account_id), ed);

		assert_current_validator_stake!(
			&validator.account_id,
			Some(TotalValidatorStake {
				total_stake ,
				..
			}) if total_stake == (550 - ed) + (450 - ed) + NOMINAL_VALUE + NOMINAL_VALUE
		);

		assert_current_contract!(
			&validator.account_id,
			&validator.account_id,
			Some(Contract { stake: Stake { currency, .. }, .. }) if currency == 550 - ed
		);

		assert_current_contract!(
			&validator.account_id,
			&delegator.account_id,
			Some(Contract { stake: Stake { currency, .. }, .. }) if currency == 450 - ed
		);
	});
}
