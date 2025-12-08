use super::*;

#[rstest]
#[case::active(true)]
#[case::inactive(false)]
fn auto_chill_works(mut ext: TestExternalities, #[case] active: bool) {
	ext.execute_with(|| {
		let validator = BindParams::default().mint().bind();

		if active {
			ValidatorSet::put(vec![validator.account_id]);
		} else {
			ValidatorSet::mutate(|s| s.retain(|v| *v != validator.account_id));
		}

		// skip sessions to enact new validator set
		next_session(); // planned
		next_session(); // in active set

		assert_validator_state!(&validator.account_id, Some(ValidatorState::Normal));

		validator.offend();
		next_session();

		assert_validator_state!(&validator.account_id, Some(ValidatorState::Faulted));

		validator.offend();
		next_session();

		let chill_idx = Session::current_index();
		assert_validator_state!(&validator.account_id, Some(ValidatorState::Chilled(s)) if s == chill_idx);

		System::assert_has_event(Event::<Test>::ValidatorChilled { validator: validator.account_id, reason: ChillReason::DoubleFault  }.into());
	});
}

#[rstest]
fn state_resets_on_reward(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let active_validator = BindParams::default().account_index(1).mint().bind();
		let inactive_validator = BindParams::default().account_index(2).mint().bind();

		ValidatorSet::put(vec![active_validator.account_id]);

		// skip sessions to enact new validator set
		next_session();
		next_session();

		assert_validator_state!(&active_validator.account_id, Some(ValidatorState::Normal));
		assert_validator_state!(&inactive_validator.account_id, Some(ValidatorState::Normal));

		active_validator.offend();
		inactive_validator.offend();
		next_session();

		assert_validator_state!(&active_validator.account_id, Some(ValidatorState::Faulted));
		assert_validator_state!(&inactive_validator.account_id, Some(ValidatorState::Faulted));

		next_session();

		// acitve validator is rewarded
		assert_validator_state!(&active_validator.account_id, Some(ValidatorState::Normal));
		assert_validator_state!(&inactive_validator.account_id, Some(ValidatorState::Faulted));
	});
}

#[rstest]
fn disqualify(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().nft_nominal_value(1000).mint();

		// Be on the verge of disqualification. After a 1% slash this should be < 80% of the initial nominal value.
		NftStakingHandler::set_nominal_value(&validator.permission_nft, 801)
			.expect("could slash nft");

		let validator = validator.bind();

		// Wait a session for the validator to become slashable
		next_session();

		validator.offend();

		next_session();

		System::assert_has_event(
			Event::<Test>::ValidatorChilled {
				validator: validator.account_id,
				reason: ChillReason::Disqualified,
			}
			.into(),
		);
	});
}
