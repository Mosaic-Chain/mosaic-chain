use std::collections::HashMap;

use crate::{mock::*, DoubleBucketMap, Event, SubsetSize, ValidatorSuperset};
use frame_support::{assert_err, assert_ok, StorageMap};
use frame_system::RawOrigin;
use sp_runtime::{AccountId32, FixedI64};

#[test]
fn select_subset_statistics() {
	new_test_ext().execute_with(|| {
		let mut subset_sizes = Vec::<usize>::new();
		let mut outlier_subset_size_cnt = 0;
		let mut validator_selection_counts: HashMap<AccountId, u32> = HashMap::new();
		let superset = <Test as ValidatorSuperset<AccountId>>::get_superset();
		for validator_id in &superset {
			validator_selection_counts.insert(validator_id.clone(), 0);
		}
		let n_rounds = 42; //Approximately one week
		for _ in 0..n_rounds {
			let subset = ValidatorSubsetSelection::select_subset(superset.clone(), false);
			let size = subset.len();
			subset_sizes.push(size);
			if (size as i64 - 250i64).abs() > 100 {
				outlier_subset_size_cnt += 1;
			}
			for validator_id in subset {
				if let Some(cnt) = validator_selection_counts.get(&validator_id) {
					validator_selection_counts.insert(validator_id, cnt + 1);
				}
			}
		}
		log::info!("Subset sizes {:?}", subset_sizes);
		//Assert subset sizes are in range [150, 350]
		assert_eq!(outlier_subset_size_cnt, 0);
		for (k, v) in validator_selection_counts.iter() {
			//assert that the distribution is near uniform
			assert!([9, 10, 11, 12].contains(v)); //Expected value is 10.5
		}
	})
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
		assert_ok!(ValidatorSubsetSelection::change_subset_size(origin, new_subset_size),);
		System::assert_last_event(Event::SubsetSizeChanged(new_subset_size).into());

		let superset = <Test as ValidatorSuperset<AccountId>>::get_superset();
		let subset = ValidatorSubsetSelection::select_subset(superset, false);
		System::assert_last_event(Event::FewerValidatorsThenSubset.into());
		assert_eq!(subset.len(), 1000, "All validators should be selected.");
		assert_eq!(System::events().len(), 2, "Two events are expected.");
	});
}
