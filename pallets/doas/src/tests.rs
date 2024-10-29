use super::*;
use sdk::{
	frame_support::{
		assert_noop, assert_ok,
		dispatch::{GetDispatchInfo, PostDispatchInfo},
		weights::Weight,
	},
	sp_runtime::{DispatchError, DispatchErrorWithPostInfo},
};

use mock::{
	new_test_ext, DoAs, DoAsCall, Logger, LoggerCall, RuntimeCall, RuntimeEvent as TestEvent,
	RuntimeOrigin, System, PRIVILEGED_ACCOUNT,
};

#[test]
fn test_setup_works() {
	new_test_ext().execute_with(|| {
		assert!(Logger::value_log().is_empty());
		assert!(Logger::account_log().is_empty());
	});
}

#[test]
fn doas_root_works() {
	new_test_ext().execute_with(|| {
		// A privileged function should work when `doas` is called with a privileged `origin`.
		let call = Box::new(RuntimeCall::Logger(LoggerCall::privileged_call {
			i: 42,
			weight: Weight::from_parts(1_000, 0),
		}));
		assert_ok!(DoAs::doas_root(RuntimeOrigin::signed(PRIVILEGED_ACCOUNT), call));
		assert_eq!(Logger::value_log(), vec![42i32]);

		// A privileged function should not work when `doas` is passed a non-privileged `origin`.
		let call = Box::new(RuntimeCall::Logger(LoggerCall::privileged_call {
			i: 42,
			weight: Weight::from_parts(1_000, 0),
		}));
		assert_noop!(DoAs::doas_root(RuntimeOrigin::signed(0), call), DispatchError::BadOrigin);
	});
}

#[test]
fn doas_root_emits_events_correctly() {
	new_test_ext().execute_with(|| {
		// Should emit event to indicate success when called with the right origin and `call` is `Ok`.
		let call = Box::new(RuntimeCall::Logger(LoggerCall::privileged_call {
			i: 42,
			weight: Weight::from_parts(1, 0),
		}));
		assert_ok!(DoAs::doas_root(RuntimeOrigin::signed(PRIVILEGED_ACCOUNT), call));
		System::assert_has_event(TestEvent::DoAs(Event::DidAsRoot { doas_result: Ok(()) }));
	});
}

#[test]
fn doas_root_unchecked_weight_basics() {
	new_test_ext().execute_with(|| {
		// A privileged function should work when `doas` is passed the right origin.
		let call = Box::new(RuntimeCall::Logger(LoggerCall::privileged_call {
			i: 42,
			weight: Weight::from_parts(1_000, 0),
		}));
		assert_ok!(DoAs::doas_root_unchecked_weight(
			RuntimeOrigin::signed(PRIVILEGED_ACCOUNT),
			call,
			Weight::from_parts(1_000, 0)
		));
		assert_eq!(Logger::value_log(), vec![42i32]);

		// A privileged function should not work when called with the wrong origin.
		let call = Box::new(RuntimeCall::Logger(LoggerCall::privileged_call {
			i: 42,
			weight: Weight::from_parts(1_000, 0),
		}));
		assert_noop!(
			DoAs::doas_root_unchecked_weight(
				RuntimeOrigin::signed(0),
				call,
				Weight::from_parts(1_000, 0)
			),
			DispatchErrorWithPostInfo {
				error: DispatchError::BadOrigin,
				post_info: PostDispatchInfo::default()
			},
		);
		// `ValueLog` is unchanged after unsuccessful call.
		assert_eq!(Logger::value_log(), vec![42i32]);

		// Controls the dispatched weight.
		let call = Box::new(RuntimeCall::Logger(LoggerCall::privileged_call {
			i: 42,
			weight: Weight::from_parts(1, 0),
		}));
		let doas_unchecked_weight_call =
			DoAsCall::doas_root_unchecked_weight { call, weight: Weight::from_parts(1_000, 0) };
		let info = doas_unchecked_weight_call.get_dispatch_info();
		assert_eq!(info.weight, Weight::from_parts(1_000, 0));
	});
}

#[test]
fn doas_root_unchecked_weight_emits_events_correctly() {
	new_test_ext().execute_with(|| {
		// Should emit event to indicate success when called with the right origin and `call` is `Ok`.
		let call = Box::new(RuntimeCall::Logger(LoggerCall::privileged_call {
			i: 42,
			weight: Weight::from_parts(1, 0),
		}));
		assert_ok!(DoAs::doas_root_unchecked_weight(
			RuntimeOrigin::signed(PRIVILEGED_ACCOUNT),
			call,
			Weight::from_parts(1_000, 0)
		));
		System::assert_has_event(TestEvent::DoAs(Event::DidAsRoot { doas_result: Ok(()) }));
	});
}

#[test]
fn doas_works() {
	new_test_ext().execute_with(|| {
		// A privileged function will not work when passed to `doas`.
		let call = Box::new(RuntimeCall::Logger(LoggerCall::privileged_call {
			i: 42,
			weight: Weight::from_parts(1_000, 0),
		}));
		assert_ok!(DoAs::doas(RuntimeOrigin::signed(PRIVILEGED_ACCOUNT), 2, call));
		assert!(Logger::value_log().is_empty());
		assert!(Logger::account_log().is_empty());

		// A non-privileged function should not work when called with the wrong origin.
		let call = Box::new(RuntimeCall::Logger(LoggerCall::non_privileged_call {
			i: 42,
			weight: Weight::from_parts(1, 0),
		}));
		assert_noop!(DoAs::doas(RuntimeOrigin::signed(0), 2, call), DispatchError::BadOrigin);

		// A non-privileged function will work when passed to `doas` with the right `origin`.
		let call = Box::new(RuntimeCall::Logger(LoggerCall::non_privileged_call {
			i: 42,
			weight: Weight::from_parts(1, 0),
		}));
		assert_ok!(DoAs::doas(RuntimeOrigin::signed(PRIVILEGED_ACCOUNT), 2, call));
		assert_eq!(Logger::value_log(), vec![42i32]);
		// The correct user makes the call within `doas`.
		assert_eq!(Logger::account_log(), vec![2]);
	});
}

#[test]
fn doas_emits_events_correctly() {
	new_test_ext().execute_with(|| {
		// A non-privileged function will work when passed to `doas` with the right `origin`.
		let call = Box::new(RuntimeCall::Logger(LoggerCall::non_privileged_call {
			i: 42,
			weight: Weight::from_parts(1, 0),
		}));
		assert_ok!(DoAs::doas(RuntimeOrigin::signed(PRIVILEGED_ACCOUNT), 2, call));
		System::assert_has_event(TestEvent::DoAs(Event::DidAs { doas_result: Ok(()) }));
	});
}
