use crate::mock::*;

use sdk::{
	frame_support, frame_system, pallet_balances, pallet_collective, pallet_membership, sp_runtime,
};

use codec::Encode;
use frame_support::{assert_ok, dispatch::GetDispatchInfo};
use sp_runtime::traits::{BlakeTwo256, Hash};

#[test]
fn members_added() {
	new_test_ext().execute_with(|| {
		assert_eq!(
			TestCouncilCollectiveMembership::members(),
			vec![account(ALICE), account(BOB), account(CHARLIE)]
		);
	});
}

#[test]
fn vote_in_dave() {
	new_test_ext().execute_with(|| {
		let proposal =
			RuntimeCall::TestCouncilCollectiveMembership(pallet_membership::Call::add_member {
				who: sp_runtime::MultiAddress::Id(account(DAVE)),
			});
		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
		let proposal_weight = proposal.get_dispatch_info().call_weight;
		let hash = BlakeTwo256::hash_of(&proposal);

		assert_ok!(TestCouncilCollective::propose(
			RuntimeOrigin::signed(account(ALICE)),
			// member threshold needed to succeed (2 yes)
			2,
			Box::new(proposal.clone()),
			proposal_len
		));

		assert_ok!(TestCouncilCollective::vote(
			RuntimeOrigin::signed(account(ALICE)),
			hash,
			0,
			true
		));
		assert_ok!(TestCouncilCollective::vote(RuntimeOrigin::signed(account(BOB)), hash, 0, true));
		assert_ok!(TestCouncilCollective::vote(
			RuntimeOrigin::signed(account(CHARLIE)),
			hash,
			0,
			false
		));

		// MotionDuration is set to 3, so we need to skip to the 4th block
		System::set_block_number(4);

		assert_ok!(TestCouncilCollective::close(
			RuntimeOrigin::signed(account(ALICE)),
			hash,
			0,
			proposal_weight,
			proposal_len,
		));

		assert_eq!(
			System::events(),
			vec![
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Proposed {
					account: account(ALICE),
					proposal_index: 0,
					proposal_hash: hash,
					threshold: 2
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Voted {
					account: account(ALICE),
					proposal_hash: hash,
					voted: true,
					yes: 1,
					no: 0
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Voted {
					account: account(BOB),
					proposal_hash: hash,
					voted: true,
					yes: 2,
					no: 0
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Voted {
					account: account(CHARLIE),
					proposal_hash: hash,
					voted: false,
					yes: 2,
					no: 1
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Closed {
					proposal_hash: hash,
					yes: 2,
					no: 1
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Approved {
					proposal_hash: hash
				})),
				record(RuntimeEvent::TestCouncilCollectiveMembership(
					pallet_membership::Event::MemberAdded
				)),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Executed {
					proposal_hash: hash,
					result: Ok(())
				})),
			]
		);
		assert_eq!(
			TestCouncilCollectiveMembership::members(),
			vec![account(ALICE), account(BOB), account(CHARLIE), account(DAVE)]
		);
	});
}

#[test]
fn not_vote_in_dave() {
	new_test_ext().execute_with(|| {
		let proposal =
			RuntimeCall::TestCouncilCollectiveMembership(pallet_membership::Call::add_member {
				who: sp_runtime::MultiAddress::Id(account(DAVE)),
			});
		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
		let proposal_weight = proposal.get_dispatch_info().call_weight;
		let hash = BlakeTwo256::hash_of(&proposal);

		assert_ok!(TestCouncilCollective::propose(
			RuntimeOrigin::signed(account(ALICE)),
			// member threshold needed to succeed (2 yes)
			2,
			Box::new(proposal.clone()),
			proposal_len
		));

		assert_ok!(TestCouncilCollective::vote(
			RuntimeOrigin::signed(account(ALICE)),
			hash,
			0,
			true
		));
		assert_ok!(TestCouncilCollective::vote(
			RuntimeOrigin::signed(account(BOB)),
			hash,
			0,
			false
		));
		assert_ok!(TestCouncilCollective::vote(
			RuntimeOrigin::signed(account(CHARLIE)),
			hash,
			0,
			false
		));

		// MotionDuration is set to 3, so we need to skip to the 4th block
		System::set_block_number(4);

		assert_ok!(TestCouncilCollective::close(
			RuntimeOrigin::signed(account(ALICE)),
			hash,
			0,
			proposal_weight,
			proposal_len,
		));

		assert_eq!(
			System::events(),
			vec![
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Proposed {
					account: account(ALICE),
					proposal_index: 0,
					proposal_hash: hash,
					threshold: 2
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Voted {
					account: account(ALICE),
					proposal_hash: hash,
					voted: true,
					yes: 1,
					no: 0
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Voted {
					account: account(BOB),
					proposal_hash: hash,
					voted: false,
					yes: 1,
					no: 1
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Voted {
					account: account(CHARLIE),
					proposal_hash: hash,
					voted: false,
					yes: 1,
					no: 2
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Closed {
					proposal_hash: hash,
					yes: 1,
					no: 2
				})),
				record(RuntimeEvent::TestCouncilCollective(
					pallet_collective::Event::Disapproved { proposal_hash: hash }
				)),
			]
		);
		assert_eq!(
			TestCouncilCollectiveMembership::members(),
			vec![account(ALICE), account(BOB), account(CHARLIE)]
		);
	});
}

#[test]
fn update_dave_balance() {
	new_test_ext().execute_with(|| {
		let proposal = RuntimeCall::Balances(pallet_balances::Call::force_set_balance {
			who: sp_runtime::MultiAddress::Id(account(DAVE)),
			new_free: 10,
		});

		let proposal = RuntimeCall::DoAs(pallet_doas::Call::doas_root { call: Box::new(proposal) });

		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
		let proposal_weight = proposal.get_dispatch_info().call_weight;
		let hash = BlakeTwo256::hash_of(&proposal);

		assert_ok!(TestCouncilCollective::propose(
			RuntimeOrigin::signed(account(ALICE)),
			2,
			Box::new(proposal.clone()),
			proposal_len
		));

		assert_ok!(TestCouncilCollective::vote(
			RuntimeOrigin::signed(account(ALICE)),
			hash,
			0,
			true
		));
		assert_ok!(TestCouncilCollective::vote(
			RuntimeOrigin::signed(account(BOB)),
			hash,
			0,
			false
		));
		assert_ok!(TestCouncilCollective::vote(
			RuntimeOrigin::signed(account(CHARLIE)),
			hash,
			0,
			true
		));

		// MotionDuration is set to 3, so we need to skip to the 4th block
		System::set_block_number(4);

		assert_ok!(TestCouncilCollective::close(
			RuntimeOrigin::signed(account(ALICE)),
			hash,
			0,
			proposal_weight,
			proposal_len,
		));
		assert_eq!(
			System::events(),
			vec![
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Proposed {
					account: account(ALICE),
					proposal_index: 0,
					proposal_hash: hash,
					threshold: 2
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Voted {
					account: account(ALICE),
					proposal_hash: hash,
					voted: true,
					yes: 1,
					no: 0
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Voted {
					account: account(BOB),
					proposal_hash: hash,
					voted: false,
					yes: 1,
					no: 1
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Voted {
					account: account(CHARLIE),
					proposal_hash: hash,
					voted: true,
					yes: 2,
					no: 1
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Closed {
					proposal_hash: hash,
					yes: 2,
					no: 1
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Approved {
					proposal_hash: hash
				})),
				record(RuntimeEvent::System(frame_system::Event::NewAccount {
					account: account(DAVE)
				})),
				record(RuntimeEvent::Balances(pallet_balances::Event::Endowed {
					account: account(DAVE),
					free_balance: 10
				})),
				record(RuntimeEvent::Balances(pallet_balances::Event::Issued { amount: 10 })),
				record(RuntimeEvent::Balances(pallet_balances::Event::BalanceSet {
					who: account(DAVE),
					free: 10
				})),
				record(RuntimeEvent::DoAs(pallet_doas::Event::DidAsRoot { doas_result: Ok(()) })),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Executed {
					proposal_hash: hash,
					result: Ok(())
				}))
			]
		);
		assert_eq!(Balances::free_balance(account(DAVE)), 10);
	});
}

#[test]
fn not_update_dave_balance() {
	new_test_ext().execute_with(|| {
		let proposal = RuntimeCall::Balances(pallet_balances::Call::force_set_balance {
			who: sp_runtime::MultiAddress::Id(account(DAVE)),
			new_free: 10,
		});

		let proposal = RuntimeCall::DoAs(pallet_doas::Call::doas_root { call: Box::new(proposal) });

		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
		let proposal_weight = proposal.get_dispatch_info().call_weight;
		let hash = BlakeTwo256::hash_of(&proposal);

		assert_ok!(TestCouncilCollective::propose(
			RuntimeOrigin::signed(account(ALICE)),
			2,
			Box::new(proposal.clone()),
			proposal_len
		));

		assert_ok!(TestCouncilCollective::vote(
			RuntimeOrigin::signed(account(ALICE)),
			hash,
			0,
			true
		));
		assert_ok!(TestCouncilCollective::vote(
			RuntimeOrigin::signed(account(BOB)),
			hash,
			0,
			false
		));
		assert_ok!(TestCouncilCollective::vote(
			RuntimeOrigin::signed(account(CHARLIE)),
			hash,
			0,
			false
		));

		// MotionDuration is set to 3, so we need to skip to the 4th block
		System::set_block_number(4);

		assert_ok!(TestCouncilCollective::close(
			RuntimeOrigin::signed(account(ALICE)),
			hash,
			0,
			proposal_weight,
			proposal_len,
		));

		assert_eq!(
			System::events(),
			vec![
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Proposed {
					account: account(ALICE),
					proposal_index: 0,
					proposal_hash: hash,
					threshold: 2
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Voted {
					account: account(ALICE),
					proposal_hash: hash,
					voted: true,
					yes: 1,
					no: 0
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Voted {
					account: account(BOB),
					proposal_hash: hash,
					voted: false,
					yes: 1,
					no: 1
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Voted {
					account: account(CHARLIE),
					proposal_hash: hash,
					voted: false,
					yes: 1,
					no: 2
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Closed {
					proposal_hash: hash,
					yes: 1,
					no: 2
				})),
				record(RuntimeEvent::TestCouncilCollective(
					pallet_collective::Event::Disapproved { proposal_hash: hash }
				))
			]
		);

		assert_eq!(Balances::free_balance(account(DAVE)), 0);
	});
}

#[test]
fn two_member_agrees_on_something_non_root() {
	new_test_ext().execute_with(|| {
		// Bob "resigning" is a root or Collective 2/3 call, member can't quit on their own
		assert_ok!(TestCouncilCollectiveMembership::remove_member(
			RuntimeOrigin::root(),
			sp_runtime::MultiAddress::Id(account(BOB))
		));

		assert_eq!(
			TestCouncilCollectiveMembership::members(),
			vec![account(ALICE), account(CHARLIE)]
		);

		let proposal =
			RuntimeCall::TestCouncilCollectiveMembership(pallet_membership::Call::add_member {
				who: sp_runtime::MultiAddress::Id(account(DAVE)),
			});
		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
		let proposal_weight = proposal.get_dispatch_info().call_weight;
		let hash = BlakeTwo256::hash_of(&proposal);

		assert_ok!(TestCouncilCollective::propose(
			RuntimeOrigin::signed(account(ALICE)),
			// member threshold needed to succeed (2 yes)
			2,
			Box::new(proposal.clone()),
			proposal_len
		));

		assert_ok!(TestCouncilCollective::vote(
			RuntimeOrigin::signed(account(ALICE)),
			hash,
			0,
			true
		));
		assert_ok!(TestCouncilCollective::vote(
			RuntimeOrigin::signed(account(CHARLIE)),
			hash,
			0,
			true
		));

		// MotionDuration is set to 3, so we need to skip to the 4th block
		System::set_block_number(4);

		assert_ok!(TestCouncilCollective::close(
			RuntimeOrigin::signed(account(ALICE)),
			hash,
			0,
			proposal_weight,
			proposal_len,
		));

		assert_eq!(
			System::events(),
			vec![
				record(RuntimeEvent::TestCouncilCollectiveMembership(
					pallet_membership::Event::MemberRemoved {}
				)),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Proposed {
					account: account(ALICE),
					proposal_index: 0,
					proposal_hash: hash,
					threshold: 2
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Voted {
					account: account(ALICE),
					proposal_hash: hash,
					voted: true,
					yes: 1,
					no: 0
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Voted {
					account: account(CHARLIE),
					proposal_hash: hash,
					voted: true,
					yes: 2,
					no: 0
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Closed {
					proposal_hash: hash,
					yes: 2,
					no: 0
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Approved {
					proposal_hash: hash
				})),
				record(RuntimeEvent::TestCouncilCollectiveMembership(
					pallet_membership::Event::MemberAdded
				)),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Executed {
					proposal_hash: hash,
					result: Ok(())
				})),
			]
		);
		assert_eq!(
			TestCouncilCollectiveMembership::members(),
			vec![account(ALICE), account(CHARLIE), account(DAVE)]
		);
	});
}

#[test]
fn two_member_disagrees_on_something_non_root() {
	new_test_ext().execute_with(|| {
		// Bob "resigning" is a root or Collective 2/3 call, member can't quit on their own
		assert_ok!(TestCouncilCollectiveMembership::remove_member(
			RuntimeOrigin::root(),
			sp_runtime::MultiAddress::Id(account(BOB))
		));

		assert_eq!(
			TestCouncilCollectiveMembership::members(),
			vec![account(ALICE), account(CHARLIE)]
		);

		let proposal =
			RuntimeCall::TestCouncilCollectiveMembership(pallet_membership::Call::add_member {
				who: sp_runtime::MultiAddress::Id(account(DAVE)),
			});
		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
		let proposal_weight = proposal.get_dispatch_info().call_weight;
		let hash = BlakeTwo256::hash_of(&proposal);

		assert_ok!(TestCouncilCollective::propose(
			RuntimeOrigin::signed(account(ALICE)),
			// member threshold needed to succeed (2 yes)
			2,
			Box::new(proposal.clone()),
			proposal_len
		));

		assert_ok!(TestCouncilCollective::vote(
			RuntimeOrigin::signed(account(ALICE)),
			hash,
			0,
			true
		));
		assert_ok!(TestCouncilCollective::vote(
			RuntimeOrigin::signed(account(CHARLIE)),
			hash,
			0,
			false
		));

		// MotionDuration is set to 3, so we need to skip to the 4th block
		System::set_block_number(4);

		assert_ok!(TestCouncilCollective::close(
			RuntimeOrigin::signed(account(ALICE)),
			hash,
			0,
			proposal_weight,
			proposal_len,
		));

		assert_eq!(
			System::events(),
			vec![
				record(RuntimeEvent::TestCouncilCollectiveMembership(
					pallet_membership::Event::MemberRemoved {}
				)),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Proposed {
					account: account(ALICE),
					proposal_index: 0,
					proposal_hash: hash,
					threshold: 2
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Voted {
					account: account(ALICE),
					proposal_hash: hash,
					voted: true,
					yes: 1,
					no: 0
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Voted {
					account: account(CHARLIE),
					proposal_hash: hash,
					voted: false,
					yes: 1,
					no: 1
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Closed {
					proposal_hash: hash,
					yes: 1,
					no: 1
				})),
				record(RuntimeEvent::TestCouncilCollective(
					pallet_collective::Event::Disapproved { proposal_hash: hash }
				)),
			]
		);
		assert_eq!(
			TestCouncilCollectiveMembership::members(),
			vec![account(ALICE), account(CHARLIE)]
		);
	});
}

#[test]
fn two_member_agrees_on_something_root() {
	new_test_ext().execute_with(|| {
		// Bob "resigning" is a root or Collective 2/3 call, member can't quit on their own
		assert_ok!(TestCouncilCollectiveMembership::remove_member(
			RuntimeOrigin::root(),
			sp_runtime::MultiAddress::Id(account(BOB))
		));

		assert_eq!(
			TestCouncilCollectiveMembership::members(),
			vec![account(ALICE), account(CHARLIE)]
		);

		let proposal = RuntimeCall::Balances(pallet_balances::Call::force_set_balance {
			who: sp_runtime::MultiAddress::Id(account(DAVE)),
			new_free: 10,
		});

		let proposal = RuntimeCall::DoAs(pallet_doas::Call::doas_root { call: Box::new(proposal) });

		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
		let proposal_weight = proposal.get_dispatch_info().call_weight;
		let hash = BlakeTwo256::hash_of(&proposal);

		assert_ok!(TestCouncilCollective::propose(
			RuntimeOrigin::signed(account(ALICE)),
			2,
			Box::new(proposal.clone()),
			proposal_len
		));

		assert_ok!(TestCouncilCollective::vote(
			RuntimeOrigin::signed(account(ALICE)),
			hash,
			0,
			true
		));
		assert_ok!(TestCouncilCollective::vote(
			RuntimeOrigin::signed(account(CHARLIE)),
			hash,
			0,
			true
		));

		// MotionDuration is set to 3, so we need to skip to the 4th block
		System::set_block_number(4);

		assert_ok!(TestCouncilCollective::close(
			RuntimeOrigin::signed(account(ALICE)),
			hash,
			0,
			proposal_weight,
			proposal_len,
		));
		assert_eq!(
			System::events(),
			vec![
				record(RuntimeEvent::TestCouncilCollectiveMembership(
					pallet_membership::Event::MemberRemoved {}
				)),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Proposed {
					account: account(ALICE),
					proposal_index: 0,
					proposal_hash: hash,
					threshold: 2
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Voted {
					account: account(ALICE),
					proposal_hash: hash,
					voted: true,
					yes: 1,
					no: 0
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Voted {
					account: account(CHARLIE),
					proposal_hash: hash,
					voted: true,
					yes: 2,
					no: 0
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Closed {
					proposal_hash: hash,
					yes: 2,
					no: 0
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Approved {
					proposal_hash: hash
				})),
				record(RuntimeEvent::System(frame_system::Event::NewAccount {
					account: account(DAVE)
				})),
				record(RuntimeEvent::Balances(pallet_balances::Event::Endowed {
					account: account(DAVE),
					free_balance: 10
				})),
				record(RuntimeEvent::Balances(pallet_balances::Event::Issued { amount: 10 })),
				record(RuntimeEvent::Balances(pallet_balances::Event::BalanceSet {
					who: account(DAVE),
					free: 10
				})),
				record(RuntimeEvent::DoAs(pallet_doas::Event::DidAsRoot { doas_result: Ok(()) })),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Executed {
					proposal_hash: hash,
					result: Ok(())
				}))
			]
		);
		assert_eq!(Balances::free_balance(account(DAVE)), 10);
	});
}

#[test]
fn two_member_disagrees_on_something_root() {
	new_test_ext().execute_with(|| {
		// Bob "resigning" is a root or Collective 2/3 call, member can't quit on their own
		assert_ok!(TestCouncilCollectiveMembership::remove_member(
			RuntimeOrigin::root(),
			sp_runtime::MultiAddress::Id(account(BOB))
		));

		assert_eq!(
			TestCouncilCollectiveMembership::members(),
			vec![account(ALICE), account(CHARLIE)]
		);

		let proposal = RuntimeCall::Balances(pallet_balances::Call::force_set_balance {
			who: sp_runtime::MultiAddress::Id(account(DAVE)),
			new_free: 10,
		});

		let proposal = RuntimeCall::DoAs(pallet_doas::Call::doas_root { call: Box::new(proposal) });

		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
		let proposal_weight = proposal.get_dispatch_info().call_weight;
		let hash = BlakeTwo256::hash_of(&proposal);

		assert_ok!(TestCouncilCollective::propose(
			RuntimeOrigin::signed(account(ALICE)),
			2,
			Box::new(proposal.clone()),
			proposal_len
		));

		assert_ok!(TestCouncilCollective::vote(
			RuntimeOrigin::signed(account(ALICE)),
			hash,
			0,
			true
		));
		assert_ok!(TestCouncilCollective::vote(
			RuntimeOrigin::signed(account(CHARLIE)),
			hash,
			0,
			false
		));

		// MotionDuration is set to 3, so we need to skip to the 4th block
		System::set_block_number(4);

		assert_ok!(TestCouncilCollective::close(
			RuntimeOrigin::signed(account(ALICE)),
			hash,
			0,
			proposal_weight,
			proposal_len,
		));

		assert_eq!(
			System::events(),
			vec![
				record(RuntimeEvent::TestCouncilCollectiveMembership(
					pallet_membership::Event::MemberRemoved {}
				)),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Proposed {
					account: account(ALICE),
					proposal_index: 0,
					proposal_hash: hash,
					threshold: 2
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Voted {
					account: account(ALICE),
					proposal_hash: hash,
					voted: true,
					yes: 1,
					no: 0
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Voted {
					account: account(CHARLIE),
					proposal_hash: hash,
					voted: false,
					yes: 1,
					no: 1
				})),
				record(RuntimeEvent::TestCouncilCollective(pallet_collective::Event::Closed {
					proposal_hash: hash,
					yes: 1,
					no: 1
				})),
				record(RuntimeEvent::TestCouncilCollective(
					pallet_collective::Event::Disapproved { proposal_hash: hash }
				))
			]
		);
		assert_eq!(Balances::free_balance(account(DAVE)), 0);
	});
}
