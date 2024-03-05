use std::collections::HashMap;

use frame_support::{assert_err, assert_ok, traits::ValidatorSet};
use frame_system::RawOrigin;
use sp_runtime::AccountId32;

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
			sp_runtime::DispatchError::BadOrigin,
		);
		assert_eq!(System::events().len(), 0, "Zero events expected.");
	});
}

#[test]
fn event_fewer_validators_than_subset() {
	new_test_ext().execute_with(|| {
		let origin: RuntimeOrigin = RawOrigin::Root.into();
		let new_subset_size = 1200;

		assert_ok!(ValidatorSubsetSelection::change_subset_size(origin, new_subset_size), ());

		System::assert_last_event(Event::SubsetSizeChanged(new_subset_size).into());

		let superset = <Test as ValidatorSet<AccountId>>::validators();
		let subset = ValidatorSubsetSelection::select_subset(superset);

		System::assert_last_event(Event::FewerValidatorsThanSubset.into());

		assert_eq!(subset.len(), 1000, "All validators should be selected.");
		assert_eq!(System::events().len(), 2, "Two events are expected.");
	});
}
