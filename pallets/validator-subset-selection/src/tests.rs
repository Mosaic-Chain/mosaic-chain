use crate::{mock::*, DoubleBucketMap, Event, SubsetSize, ValidatorSuperset};
use frame_support::{assert_err, assert_ok, StorageMap};
use frame_system::RawOrigin;
use sp_runtime::{AccountId32, FixedI64};

#[test]
fn select_subset() {
	new_test_ext().execute_with(|| {
		let superset = <Test as ValidatorSuperset<AccountId>>::get_superset();
		/*DoubleBucketMap::<Test>::insert(
			superset[0].clone(),
			(FixedI64::from_rational(1, 2), FixedI64::from_rational(1, 2)),
		);*/
	})
}

#[test]
fn root_changes_subset_size() {
	new_test_ext().execute_with(|| {
		assert_eq!(ValidatorSubsetSelection::subset_size(), 3);
		let origin = RawOrigin::Root.into();
		let new_subset_size = 5;
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
		let new_subset_size = 5;
		assert_err!(
			ValidatorSubsetSelection::change_subset_size(origin, new_subset_size),
			sp_runtime::DispatchError::BadOrigin
		);
		assert_eq!(System::events().len(), 0);
	});
}
