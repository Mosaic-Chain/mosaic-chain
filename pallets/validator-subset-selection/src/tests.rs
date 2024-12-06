use std::collections::HashMap;

use sdk::{
	frame_support::traits::ValidatorSet,
	pallet_session::SessionManager,
	sp_core::Get,
	sp_runtime::{traits::Zero, FixedI64},
};

use crate::{
	mock::*, AvgSessionLength, CurrentSessionEnd, CurrentSessionLength, DoubleBucketMap, Event,
	NextSessionEnd,
};

#[test]
fn select_subset_statistics() {
	new_test_ext(2000, 200).execute_with(|| {
		let mut validator_selection_counts: HashMap<AccountId, u32> = HashMap::new();
		let mut sum_session_len = 0;

		let superset = Superset::validators();
		let n_rounds = 168; // Approximately one week (1h session, 6s block time)

		for _ in 0..n_rounds {
			let subset =
				ValidatorSubsetSelection::select_subset(superset.clone(), SubsetSize::get());
			let size = subset.len();

			assert!((135..=265).contains(&size), "subset size of {size} is out of expected range");

			let length = ValidatorSubsetSelection::session_length(size as u64);

			// 45m - 1h15m
			assert!(
				(450..=750).contains(&length),
				"calculated session length of {length} is out of expected range"
			);

			sum_session_len += length;

			for validator_id in subset {
				*validator_selection_counts.entry(validator_id).or_default() += 1;
			}
		}

		let avg_session_len = sum_session_len / n_rounds;
		// 55m - 1h05m
		assert!(
			(550..=650).contains(&avg_session_len),
			"avg session length of {avg_session_len} is out of expected range",
		);

		for validator in superset {
			//assert that the distribution is near uniform
			let value = validator_selection_counts
				.get(&validator)
				.expect("validator was not selected at all");
			assert!(
				(15..20).contains(value), //Expected value is 17
				"validator selected count of {value} is out of expected range"
			);
		}
	});
}

#[test]
fn fewer_validators_than_subset_event_works() {
	new_test_ext(10, 15).execute_with(|| {
		let superset = Superset::validators();
		let mut subset =
			ValidatorSubsetSelection::select_subset(superset.clone(), SubsetSize::get());
		subset.sort_unstable();

		System::assert_has_event(Event::FewerValidatorsThanSubset.into());
		assert_eq!(superset, subset);
	});
}

#[test]
fn empty_vec_for_select_subset() {
	new_test_ext(15, 10).execute_with(|| {
		let empty_vec = Vec::new();
		let result = ValidatorSubsetSelection::select_subset(empty_vec.clone(), SubsetSize::get());
		assert_eq!(empty_vec, result);
		System::assert_last_event(Event::FewerValidatorsThanSubset.into());
	});
}

#[test]
fn empty_subset_event_works() {
	new_test_ext(15, 10).execute_with(|| {
		let validators: Vec<AccountId> = Superset::validators();

		for v in &validators {
			DoubleBucketMap::<Test>::insert(v, (FixedI64::zero(), FixedI64::zero()));
		}

		ValidatorSubsetSelection::select_subset(validators, SubsetSize::get());

		System::assert_last_event(Event::EmptySubset.into());
	});
}

#[test]
fn new_session_genesis_works() {
	new_test_ext(15, 10).execute_with(|| {
		let result0 = ValidatorSubsetSelection::new_session_genesis(0).unwrap();

		let len0 = ValidatorSubsetSelection::session_length(result0.len() as u64);
		assert_eq!(CurrentSessionLength::<Test>::get(), len0 + 1);
		assert_eq!(CurrentSessionEnd::<Test>::get(), len0);

		System::assert_last_event(
			Event::SubsetSelected {
				validator_subset: result0.clone(),
				session_start: 0,
				session_end: len0,
				session_index: 0,
			}
			.into(),
		);

		let result1 = ValidatorSubsetSelection::new_session_genesis(1).unwrap();

		// Rotation not yet happened as session 1 is planned immediately after session 0.
		assert_eq!(CurrentSessionLength::<Test>::get(), len0 + 1);
		assert_eq!(CurrentSessionEnd::<Test>::get(), len0);

		let len1 = ValidatorSubsetSelection::session_length(result1.len() as u64);

		System::assert_last_event(
			Event::SubsetSelected {
				validator_subset: result1.clone(),
				session_start: len0 + 1,
				session_end: len0 + len1,
				session_index: 1,
			}
			.into(),
		);
	});
}

#[test]
fn end_session_works() {
	new_test_ext(15, 10).execute_with(|| {
		let session_index = 1;
		let next_session_end = NextSessionEnd::<Test>::get();
		let current_session_end = CurrentSessionEnd::<Test>::get();
		let expected = next_session_end - current_session_end;

		ValidatorSubsetSelection::end_session(session_index);

		assert_eq!(expected, CurrentSessionLength::<Test>::get());
		assert_eq!(next_session_end, CurrentSessionEnd::<Test>::get());
	});
}

#[test]
fn new_session_works() {
	new_test_ext(15, 10).execute_with(|| {
		let session_index = 2;
		let current_session_length = CurrentSessionLength::<Test>::get();
		let avg = AvgSessionLength::<Test>::get();
		let current_session_end = CurrentSessionEnd::<Test>::get();

		let result = ValidatorSubsetSelection::new_session(session_index).unwrap();

		let new_session_length = ValidatorSubsetSelection::session_length(result.len() as u64);
		let new_session_end = current_session_length + new_session_length;

		let expected_avg_session_length = ValidatorSubsetSelection::new_avg_session_length(
			session_index,
			new_session_length,
			avg,
		);

		let next_session_end = NextSessionEnd::<Test>::get();
		assert_eq!(new_session_end, next_session_end);

		let avg_session_length = AvgSessionLength::<Test>::get();
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
