use super::*;
use crate::mock::*;
use sdk::{frame_support, sp_core, sp_runtime};

use sp_core::offchain::{
	testing::{TestOffchainExt, TestTransactionPoolExt},
	OffchainDbExt, OffchainWorkerExt, TransactionPoolExt,
};

use sp_runtime::testing::UintAuthorityId;

use frame_support::{assert_noop, dispatch};

#[test]
fn test_unresponsiveness_slash_fraction() {
	let dummy_offence =
		UnresponsivenessOffence { session_index: 0, validator_set_count: 50, offenders: vec![()] };
	// A single case of unresponsiveness is not slashed.
	assert_eq!(dummy_offence.slash_fraction(1), Perbill::zero());

	assert_eq!(
		dummy_offence.slash_fraction(5),
		Perbill::zero(), // 0%
	);

	assert_eq!(
		dummy_offence.slash_fraction(7),
		Perbill::from_parts(4200000), // 0.42%
	);

	// One third offline should be punished around 5%.
	assert_eq!(
		dummy_offence.slash_fraction(17),
		Perbill::from_parts(46200000), // 4.62%
	);
}

#[allow(clippy::type_complexity)]
fn assert_single_offence(
	offences: &[(Vec<u64>, UnresponsivenessOffence<(u64, u64)>)],
	session_index: SessionIndex,
	validator_set_count: u32,
	expected_offenders: &[(u64, u64)],
) {
	let offence = &offences[0].1;
	let mut offenders = offence.offenders.clone();
	offenders.sort_by_key(|(i, _)| *i);

	assert_eq!(offence.session_index, session_index);
	assert_eq!(offence.validator_set_count, validator_set_count);
	assert_eq!(offenders, expected_offenders);
}

#[test]
fn should_report_offline_validators() {
	new_test_ext().execute_with(|| {
		// given
		let block = 1;
		System::set_block_number(block);
		// buffer new validators
		advance_session();
		// enact the change and buffer another one
		let validators = vec![1, 2, 3, 4, 5, 6];
		Validators::mutate(|l| *l = Some(validators.clone()));
		advance_session();

		// when
		// we end current session and start the next one
		advance_session();

		// then
		let offences = Offences::take();
		assert_single_offence(&offences, 2, 3, &[(1, 1), (2, 2), (3, 3)]);

		// should not report when heartbeat is sent
		for v in validators.into_iter().take(4) {
			heartbeat(block, 3, v.into()).unwrap();
		}
		advance_session();

		// then
		let offences = Offences::take();
		assert_single_offence(&offences, 3, 6, &[(5, 5), (6, 6)]);
	});
}

fn heartbeat(
	block_number: u64,
	session_index: u32,
	key: UintAuthorityId,
) -> dispatch::DispatchResult {
	let heartbeat = Heartbeat { block_number, session_index, key: key.clone() };
	let signature = key.sign(&heartbeat.encode()).unwrap();

	ImOnline::pre_dispatch(&crate::Call::heartbeat {
		heartbeat: heartbeat.clone(),
		signature: signature.clone(),
	})
	.map_err(<&'static str>::from)?;
	ImOnline::heartbeat(RuntimeOrigin::none(), heartbeat, signature)
}

#[test]
fn should_mark_online_validator_when_heartbeat_is_received() {
	new_test_ext().execute_with(|| {
		advance_session();
		// given
		Validators::mutate(|l| *l = Some(vec![1, 2, 3, 4, 5, 6]));
		assert_eq!(Session::validators(), Vec::<u64>::new());
		// enact the change and buffer another one
		advance_session();

		assert_eq!(Session::current_index(), 2);
		assert_eq!(Session::validators(), vec![1, 2, 3]);

		assert!(!ImOnline::is_online(&1.into()));
		assert!(!ImOnline::is_online(&2.into()));
		assert!(!ImOnline::is_online(&3.into()));

		// when
		heartbeat(1, 2, 1.into()).unwrap();

		// then
		assert!(ImOnline::is_online(&1.into()));
		assert!(!ImOnline::is_online(&2.into()));
		assert!(!ImOnline::is_online(&3.into()));

		// and when
		heartbeat(1, 2, 3.into()).unwrap();

		// then
		assert!(ImOnline::is_online(&1.into()));
		assert!(!ImOnline::is_online(&2.into()));
		assert!(ImOnline::is_online(&3.into()));
	});
}

#[test]
fn should_handle_heartbeats_of_validator_outside_active_set() {
	new_test_ext().execute_with(|| {
		Validators::mutate(|l| *l = Some(vec![1, 2, 3]));
		advance_session(); // [1, 2, 3] scheduled
		MockSlashableValidators::mutate(|v| *v = Some(vec![1, 2, 3, 4, 5, 6]));
		advance_session(); // [1, 2, 3] active and [1, 2, 3, 4, 5, 6] slashable

		assert_eq!(Session::current_index(), 2);
		assert_eq!(Session::validators(), vec![1, 2, 3]);

		assert!(!ImOnline::is_online(&4.into()));
		assert!(!ImOnline::is_online(&5.into()));
		assert!(!ImOnline::is_online(&6.into()));

		heartbeat(1, 2, 4.into()).unwrap();
		heartbeat(1, 2, 6.into()).unwrap();

		assert!(ImOnline::is_online(&4.into()));
		assert!(!ImOnline::is_online(&5.into()));
		assert!(ImOnline::is_online(&6.into()));

		advance_session();

		let offences = Offences::take();
		assert_single_offence(&offences, 2, 6, &[(1, 1), (2, 2), (3, 3), (5, 5)]);
	});
}

#[test]
fn late_heartbeat_should_fail() {
	new_test_ext().execute_with(|| {
		advance_session();
		// given
		Validators::mutate(|l| *l = Some(vec![1, 2, 3, 4, 5, 6]));
		assert_eq!(Session::validators(), Vec::<u64>::new());
		// enact the change and buffer another one
		advance_session();

		assert_eq!(Session::current_index(), 2);
		assert_eq!(Session::validators(), vec![1, 2, 3]);

		// when
		assert_noop!(heartbeat(1, 3, 1.into()), "Transaction is outdated");
		assert_noop!(heartbeat(1, 1, 1.into()), "Transaction is outdated");
	});
}

#[test]
fn should_generate_heartbeats() {
	let mut ext = new_test_ext();
	let (offchain, _state) = TestOffchainExt::new();
	let (pool, state) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		// given
		let block = 1;
		System::set_block_number(block);
		UintAuthorityId::set_all_keys(vec![1, 2, 3]);
		// buffer new validators
		Session::rotate_session();
		// enact the change and buffer another one
		Validators::mutate(|l| *l = Some(vec![1, 2, 3, 4, 5, 6]));
		Session::rotate_session();

		// when
		ImOnline::offchain_worker(block);

		// then
		let transactions = state.read().transactions.clone();
		assert_eq!(transactions.len(), 3);

		let heartbeats: Vec<_> = transactions
			.into_iter()
			.map(|tx| {
				let ex: Extrinsic = Decode::decode(&mut &*tx).unwrap();
				match ex.function {
					crate::mock::RuntimeCall::ImOnline(crate::Call::heartbeat {
						heartbeat,
						..
					}) => heartbeat,
					e => panic!("Unexpected call: {e:?}"),
				}
			})
			.collect();

		for key in 1..=3 {
			assert!(heartbeats.contains(&Heartbeat {
				block_number: block,
				session_index: 2,
				key: key.into()
			}));
		}
	});
}

#[test]
fn should_cleanup_received_heartbeats_on_session_end() {
	new_test_ext().execute_with(|| {
		advance_session();

		Validators::mutate(|l| *l = Some(vec![1, 2, 3]));
		assert_eq!(Session::validators(), Vec::<u64>::new());

		// enact the change and buffer another one
		advance_session();

		assert_eq!(Session::current_index(), 2);
		assert_eq!(Session::validators(), vec![1, 2, 3]);

		// send an heartbeat from authority id 0 at session 2
		heartbeat(1, 2, 1.into()).unwrap();

		// the heartbeat is stored
		assert!(super::pallet::ReceivedHeartbeats::<Runtime>::get(2, 1).is_some());

		advance_session();

		// after the session has ended we have already processed the heartbeat
		// message, so any messages received on the previous session should have
		// been pruned.
		assert!(super::pallet::ReceivedHeartbeats::<Runtime>::get(2, 1).is_none());
	});
}

#[test]
fn should_mark_online_validator_when_block_is_authored() {
	use pallet_authorship::EventHandler;

	new_test_ext().execute_with(|| {
		advance_session();
		// given
		Validators::mutate(|l| *l = Some(vec![1, 2, 3, 4, 5, 6]));
		assert_eq!(Session::validators(), Vec::<u64>::new());
		// enact the change and buffer another one
		advance_session();

		assert_eq!(Session::current_index(), 2);
		assert_eq!(Session::validators(), vec![1, 2, 3]);

		for i in 1..=3 {
			assert!(!ImOnline::is_online(&i.into()));
		}

		// when
		ImOnline::note_author(1);

		// then
		assert!(ImOnline::is_online(&1.into()));
		assert!(!ImOnline::is_online(&2.into()));
		assert!(!ImOnline::is_online(&3.into()));
	});
}

#[test]
fn should_not_send_a_report_if_already_online() {
	use pallet_authorship::EventHandler;

	let mut ext = new_test_ext();
	let (offchain, _state) = TestOffchainExt::new();
	let (pool, pool_state) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		advance_session();
		// given
		Validators::mutate(|l| *l = Some(vec![1, 2, 3, 4, 5, 6]));
		assert_eq!(Session::validators(), Vec::<u64>::new());
		// enact the change and buffer another one
		advance_session();
		assert_eq!(Session::current_index(), 2);
		assert_eq!(Session::validators(), vec![1, 2, 3]);
		ImOnline::note_author(2);
		ImOnline::note_author(3);

		// when
		UintAuthorityId::set_all_keys(vec![1, 2, 3]);
		// we expect 2 errors, since 2 authority is already online.
		let already_online = ImOnline::send_heartbeats(4)
			.unwrap()
			.filter(|res| matches!(res, Err(OffchainErr::AlreadyOnline)))
			.count();

		assert_eq!(already_online, 2);

		// then
		// we should only produce 1 heartbeat.
		assert_eq!(pool_state.read().transactions.len(), 1);
		let transaction = pool_state.write().transactions.pop().unwrap();
		// check stuff about the transaction.
		let ex: Extrinsic = Decode::decode(&mut &*transaction).unwrap();
		let heartbeat = match ex.function {
			crate::mock::RuntimeCall::ImOnline(crate::Call::heartbeat { heartbeat, .. }) => {
				heartbeat
			},
			e => panic!("Unexpected call: {e:?}"),
		};

		assert_eq!(heartbeat, Heartbeat { block_number: 4, session_index: 2, key: 1.into() });
	});
}

#[test]
fn should_handle_missing_progress_estimates() {
	let mut ext = new_test_ext();
	let (offchain, _state) = TestOffchainExt::new();
	let (pool, state) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		let block = 1;

		System::set_block_number(block);
		UintAuthorityId::set_all_keys(vec![1, 2, 3]);

		// buffer new validators
		Session::rotate_session();

		// enact the change and buffer another one
		Validators::mutate(|l| *l = Some(vec![1, 2, 3]));
		Session::rotate_session();

		// we will return `None` on the next call to `estimate_current_session_progress`
		// and the offchain worker should fallback to checking `HeartbeatAfter`
		MockCurrentSessionProgress::mutate(|p| *p = Some(None));
		ImOnline::offchain_worker(block);

		assert_eq!(state.read().transactions.len(), 3);
	});
}

#[test]
fn should_handle_non_linear_session_progress() {
	// NOTE: this is the reason why we started using `EstimateNextSessionRotation` to figure out if
	// we should send a heartbeat, it's possible that between successive blocks we progress through
	// the session more than just one block increment (in BABE session length is defined in slots,
	// not block numbers).

	let mut ext = new_test_ext();
	let (offchain, _state) = TestOffchainExt::new();
	let (pool, _) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		UintAuthorityId::set_all_keys(vec![1, 2, 3]);

		// buffer new validator
		Session::rotate_session();

		// mock the session length as being 10 blocks long,
		// enact the change and buffer another one
		Validators::mutate(|l| *l = Some(vec![1, 2, 3]));

		// mock the session length has being 10 which should make us assume the fallback for half
		// session will be reached by block 5.
		MockAverageSessionLength::mutate(|p| *p = Some(10));

		Session::rotate_session();

		// if we don't have valid results for the current session progress then
		// we'll fallback to `HeartbeatAfter` and only heartbeat on block 5.
		MockCurrentSessionProgress::mutate(|p| *p = Some(None));
		assert_eq!(ImOnline::send_heartbeats(2).err(), Some(OffchainErr::TooEarly));

		MockCurrentSessionProgress::mutate(|p| *p = Some(None));
		assert!(ImOnline::send_heartbeats(5).ok().is_some());

		// if we have a valid current session progress then we'll heartbeat as soon
		// as we're past 80% of the session regardless of the block number
		MockCurrentSessionProgress::mutate(|p| *p = Some(Some(Permill::from_percent(81))));

		assert!(ImOnline::send_heartbeats(2).ok().is_some());
	});
}

#[test]
fn test_does_not_heartbeat_early_in_the_session() {
	let mut ext = new_test_ext();
	let (offchain, _state) = TestOffchainExt::new();
	let (pool, _) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		// mock current session progress as being 5%. we only randomly start
		// heartbeating after 10% of the session has elapsed.
		MockCurrentSessionProgress::mutate(|p| *p = Some(Some(Permill::from_float(0.05))));
		assert_eq!(ImOnline::send_heartbeats(2).err(), Some(OffchainErr::TooEarly));
	});
}

#[test]
fn test_probability_of_heartbeating_increases_with_session_progress() {
	let mut ext = new_test_ext();
	let (offchain, state) = TestOffchainExt::new();
	let (pool, _) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		let set_test = |progress, random: f64| {
			// the average session length is 100 blocks, therefore the residual
			// probability of sending a heartbeat is 1%
			MockAverageSessionLength::mutate(|p| *p = Some(100));
			MockCurrentSessionProgress::mutate(|p| *p = Some(Some(Permill::from_float(progress))));

			let mut seed = [0u8; 32];
			let encoded = ((random * Permill::ACCURACY as f64) as u32).encode();
			seed[0..4].copy_from_slice(&encoded);
			state.write().seed = seed;
		};

		let assert_too_early = |progress, random| {
			set_test(progress, random);
			assert_eq!(ImOnline::send_heartbeats(2).err(), Some(OffchainErr::TooEarly));
		};

		let assert_heartbeat_ok = |progress, random| {
			set_test(progress, random);
			assert!(ImOnline::send_heartbeats(2).ok().is_some());
		};

		assert_too_early(0.05, 1.0);

		// assert_too_early(0.1, 0.1);
		// assert_too_early(0.1, 0.011);
		assert_heartbeat_ok(0.1, 0.010);

		// assert_too_early(0.4, 0.015);
		assert_heartbeat_ok(0.4, 0.014);

		// assert_too_early(0.5, 0.026);
		assert_heartbeat_ok(0.5, 0.025);

		// assert_too_early(0.6, 0.057);
		assert_heartbeat_ok(0.6, 0.056);

		// assert_too_early(0.65, 0.086);
		assert_heartbeat_ok(0.65, 0.085);

		// assert_too_early(0.7, 0.13);
		assert_heartbeat_ok(0.7, 0.12);

		// assert_too_early(0.75, 0.19);
		assert_heartbeat_ok(0.75, 0.18);
	});
}
