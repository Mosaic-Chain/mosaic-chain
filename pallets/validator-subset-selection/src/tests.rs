use std::collections::HashMap;

use sdk::{
	frame_support::{assert_err, assert_ok, traits::ValidatorSet},
	frame_system::RawOrigin,
	pallet_session::SessionManager,
	sp_runtime::{AccountId32, DispatchError},
};

use crate::{mock::*, Event};

#[test]
fn select_subset_statistics() {
	new_test_ext().execute_with(|| {
		let mut validator_selection_counts: HashMap<AccountId, u32> = HashMap::new();
		let superset = <Test as ValidatorSet<AccountId>>::validators();

		// (7 * 24* 60 * 60) / (12s * ~250 block)
		let n_rounds = 200; //Approximately one week

		for _ in 0..n_rounds {
			let subset = ValidatorSubsetSelection::select_subset(superset.clone());
			let size = subset.len();

			// Why 100?
			assert!(size.abs_diff(250) <= 100, "subset size is out of expected range");

			for validator_id in subset {
				*validator_selection_counts.entry(validator_id).or_default() += 1;
			}
		}

		for validator in superset {
			//assert that the distribution is near uniform
			let value = validator_selection_counts
				.get(&validator)
				.expect("validator was not selected at all");
			assert!((42..=57).contains(value)); //Expected value is 50
		}
	});
}

#[test]
fn root_changes_subset_size() {
	new_test_ext().execute_with(|| {
		assert_eq!(ValidatorSubsetSelection::subset_size(), 250);

		let origin = RawOrigin::Root.into();
		let new_subset_size = 176;

		assert_ok!(ValidatorSubsetSelection::change_subset_size(origin, new_subset_size));
		assert_eq!(ValidatorSubsetSelection::subset_size(), new_subset_size);

		System::assert_last_event(Event::SubsetSizeChanged(new_subset_size).into());

		assert_eq!(System::events().len(), 1);
	});
}

#[test]
fn signed_changes_subset_size() {
	new_test_ext().execute_with(|| {
		let account_id = AccountId32::new([0; 32]);
		let origin: RuntimeOrigin = RuntimeOrigin::signed(account_id);
		let new_subset_size = 247;

		assert_err!(
			ValidatorSubsetSelection::change_subset_size(origin, new_subset_size),
			DispatchError::BadOrigin,
		);
		assert_eq!(System::events().len(), 0, "Zero events expected.");
	});
}

#[test]
fn event_fewer_validators_than_subset() {
	new_test_ext().execute_with(|| {
		let origin: RuntimeOrigin = RawOrigin::Root.into();
		let new_subset_size = 1200;

		assert_ok!(ValidatorSubsetSelection::change_subset_size(origin, new_subset_size));

		System::assert_last_event(Event::SubsetSizeChanged(new_subset_size).into());

		let superset = <Test as ValidatorSet<AccountId>>::validators();
		let subset = ValidatorSubsetSelection::select_subset(superset);

		System::assert_last_event(Event::FewerValidatorsThanSubset.into());

		assert_eq!(subset.len(), 1000, "All validators should be selected.");
		assert_eq!(System::events().len(), 2, "Two events are expected.");
	});
}

#[test]
fn empty_vec_for_select_subset() {
	new_test_ext().execute_with(|| {
		let empty_vec = Vec::new();
		let result = ValidatorSubsetSelection::select_subset(empty_vec.clone());
		assert_eq!(empty_vec, result);
		System::assert_last_event(Event::FewerValidatorsThanSubset.into());
	});
}

#[test]
fn empty_subset_event_should_work() {
	new_test_ext().execute_with(|| {
		// Changing the size so the random buckets won't be 1 during the 1st call
		// which results in the selected_subset to be empty

		// mean which will be added to the buckets will be 2/100 (1/50)
		let origin: RuntimeOrigin = RawOrigin::Root.into();
		let new_subset_size = 2;

		assert_ok!(ValidatorSubsetSelection::change_subset_size(origin, new_subset_size));

		let validators: Vec<AccountId> = (0..50).map(account).collect();

		ValidatorSubsetSelection::select_subset(validators);

		System::assert_last_event(Event::EmptySubset.into());
	});
}

#[test]
#[should_panic]
fn session_index_not_zero_or_one_should_panic() {
	new_test_ext().execute_with(|| {
		// anything besides 1 or 0 is good for here, because this panics
		// if the session_index is not 0 or 1
		ValidatorSubsetSelection::new_session_genesis(2);
	});
}

#[test]
fn new_session_genesis_should_work() {
	new_test_ext().execute_with(|| {
		let session_index = 0;
		let superset = <Test as ValidatorSet<AccountId>>::validators();
		let subset_size = ValidatorSubsetSelection::subset_size() as usize;
		let selected_subset = if superset.len() > subset_size {
			superset[0..subset_size].to_owned()
		} else {
			superset
		};
		let current_subset_size = (selected_subset.len() as u32).into();
		let new_session_start = 0;
		let new_session_end = ValidatorSubsetSelection::session_length(current_subset_size);

		let result = ValidatorSubsetSelection::new_session_genesis(session_index);

		assert_eq!(ValidatorSubsetSelection::current_session_length(), new_session_end);
		assert_eq!(ValidatorSubsetSelection::current_session_end(), new_session_end);

		System::assert_last_event(
			Event::SubsetSelected {
				validator_subset: selected_subset.clone(),
				session_start: new_session_start,
				session_end: new_session_end,
				session_index,
			}
			.into(),
		);

		assert_eq!(result.unwrap(), selected_subset);
	});
}

#[test]
fn end_session_should_work() {
	new_test_ext().execute_with(|| {
		let session_index = 1;
		let next_session_end = ValidatorSubsetSelection::next_session_end();
		let current_session_end = ValidatorSubsetSelection::current_session_end();
		let expected = next_session_end - current_session_end;

		ValidatorSubsetSelection::end_session(session_index);

		assert_eq!(expected, ValidatorSubsetSelection::current_session_length());
		assert_eq!(next_session_end, ValidatorSubsetSelection::current_session_end());
	});
}

#[test]
fn new_session_should_work() {
	new_test_ext().execute_with(|| {
		let session_index = 2;
		let current_session_length = ValidatorSubsetSelection::current_session_length();
		let avg = ValidatorSubsetSelection::avg_session_length();
		let current_session_end = ValidatorSubsetSelection::current_session_end();

		let result = ValidatorSubsetSelection::new_session(session_index).unwrap();

		let new_session_length = ValidatorSubsetSelection::session_length(result.len() as u64);
		let new_session_end = current_session_length + new_session_length;

		let expected_avg_session_length = ValidatorSubsetSelection::new_avg_session_length(
			session_index,
			new_session_length,
			avg,
		);

		let next_session_end = ValidatorSubsetSelection::next_session_end();
		assert_eq!(new_session_end, next_session_end);

		let avg_session_length = ValidatorSubsetSelection::avg_session_length();
		assert_eq!(expected_avg_session_length, avg_session_length);

		System::assert_last_event(
			Event::SubsetSelected {
				validator_subset: result.clone(),
				session_start: current_session_end + 1,
				session_end: new_session_end,
				session_index,
			}
			.into(),
		);
	});
}
