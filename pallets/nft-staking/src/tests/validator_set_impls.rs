use super::*;

#[rstest]
fn selectable_validators(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let normal = BindParams::default().account_index(0).mint().bind();
		let faulted = BindParams::default().account_index(1).mint().bind();
		let chilled = BindParams::default().account_index(2).mint().bind();

		ValidatorStates::<Test>::mutate_extant(&faulted.account_id, |state| {
			*state = ValidatorState::Faulted;
		});

		ValidatorStates::<Test>::mutate_extant(&chilled.account_id, |state| {
			*state = ValidatorState::Chilled(0);
		});

		let selectable = SelectableValidators::<Test>::validators();
		assert!(selectable.contains(&normal.account_id));
		assert!(selectable.contains(&faulted.account_id));
		assert!(!selectable.contains(&chilled.account_id));
	});
}

#[rstest]
#[case::normal(ValidatorState::Normal)]
#[case::faulted(ValidatorState::Faulted)]
fn slashable_validators(mut ext: TestExternalities, #[case] not_chilled_state: ValidatorState) {
	ext.execute_with(|| {
		let normal = BindParams::default().account_index(0).mint().bind();
		let chilled = BindParams::default().account_index(1).mint().bind();
		let selected = BindParams::default().account_index(2).mint().bind();
		let active = BindParams::default().account_index(3).mint().bind();
		let chilled_selected = BindParams::default().account_index(4).mint().bind();
		let chilled_active = BindParams::default().account_index(5).mint().bind();

		ValidatorStates::<Test>::mutate_extant(&chilled.account_id, |state| {
			*state = ValidatorState::Chilled(0);
		});

		ValidatorStates::<Test>::mutate_extant(&chilled_selected.account_id, |state| {
			*state = ValidatorState::Chilled(0);
		});

		ValidatorStates::<Test>::mutate_extant(&chilled_active.account_id, |state| {
			*state = ValidatorState::Chilled(0);
		});

		ValidatorStates::<Test>::mutate_extant(&normal.account_id, |state| {
			*state = not_chilled_state;
		});

		ValidatorStates::<Test>::mutate_extant(&selected.account_id, |state| {
			*state = not_chilled_state;
		});

		ValidatorStates::<Test>::mutate_extant(&active.account_id, |state| {
			*state = not_chilled_state;
		});

		// Not yet committed contracts
		// To be an active or selected validator the contract needs to be already commited
		// So we are only really testing normal and chilled.
		let slashable = SlashableValidators::<Test>::validators();
		assert!(slashable.is_empty());

		// Committed contracts
		ValidatorSet::set(vec![active.account_id.clone(), chilled_active.account_id.clone()]);

		next_session(); // actives are now selected, contracts are committed

		ValidatorSet::set(vec![selected.account_id.clone(), chilled_selected.account_id.clone()]);

		next_session(); // actives are now active, selected are selected

		let slashable = SlashableValidators::<Test>::validators();

		assert!(slashable.contains(&normal.account_id));
		assert!(!slashable.contains(&chilled.account_id));
		assert!(slashable.contains(&selected.account_id));
		assert!(slashable.contains(&active.account_id));
		assert!(slashable.contains(&chilled_selected.account_id));
		assert!(slashable.contains(&chilled_active.account_id));
	});
}
