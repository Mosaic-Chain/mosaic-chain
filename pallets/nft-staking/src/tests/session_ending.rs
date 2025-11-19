use crate::{
	session_ending::{reward, slash, Context, SessionEnding, State},
	types::{StagingLayer, StorageLayer},
	Contracts, CurrentStagingLayer, InverseSlashes, SessionEndings,
};

use super::*;

use frame_support::{
	traits::{OnFinalize, OnIdle, OnInitialize},
	weights::Weight,
};

const REWARDED_ACC: AccountId = 1;
const SLASHED_ACC: AccountId = 2;
const SLASH: u32 = 20;
const REWARD: Balance = 1000;

fn next_session_without_on_idle() {
	// skip to next session (block) without calling `on_idle`
	let block_number = System::block_number();
	AllPalletsWithSystem::on_finalize(block_number);
	System::reset_events();
	System::set_block_number(block_number + 1);
	AllPalletsWithSystem::on_initialize(block_number + 1);
}

fn setup_session_ending() {
	let rewarded_validator = BindParams::default()
		.account_index(REWARDED_ACC)
		.permission(PermissionType::DPoS)
		.mint()
		.bind();

	let slashed_validator = BindParams::default()
		.account_index(SLASHED_ACC)
		.permission(PermissionType::DPoS)
		.mint()
		.bind();

	// validator in active set
	ValidatorSet::set(vec![rewarded_validator.account_id, slashed_validator.account_id]);
	next_session(); // queued
	next_session(); // active

	InverseSlashes::<Test>::insert(slashed_validator.account_id, Perbill::from_percent(SLASH));
	SessionReward::set(REWARD);

	next_session_without_on_idle();
}

#[rstest]
fn session_ending_process(mut ext: TestExternalities) {
	ext.execute_with(|| {
		setup_session_ending();

		let session_endings = SessionEndings::<Test>::get();
		assert_eq!(session_endings.len(), 1);

		// initial state is correct
		assert!(matches!(
			&session_endings[0],
			SessionEnding {
				state: State::Reward {
					sweep: reward::Sweep::CalculateTotalStake,
					context: reward::SweepContext {
						remaining_validators,
						session_reward: 1000
					}
				},
				context: Context {
					session_index: 2,
					slash_context: slash::SweepContext {
						remaining_offenders
					}
				}
			} if remaining_validators == &vec![REWARDED_ACC]
			&& remaining_offenders == &vec![(SLASHED_ACC, Perbill::from_percent(100 - SLASH))]));

		SessionEndingLog::reset();

		// complete session ending
		AllPalletsWithSystem::on_idle(42, Weight::MAX);

		assert_eq!(
			&SessionEndingLog::entries(),
			&[
				SessionEndingLogEntry::OnIdle,
				SessionEndingLogEntry::CalculateTotalStake,
				SessionEndingLogEntry::MaybeResetValidatorState,
				SessionEndingLogEntry::RewardContract,
				SessionEndingLogEntry::DepositValidatorReward,
				SessionEndingLogEntry::DepositContribution,
				SessionEndingLogEntry::ChillIfFaulted,
				SessionEndingLogEntry::SlashContract,
				SessionEndingLogEntry::ChillIfDisqualified,
				SessionEndingLogEntry::RotateStagingLayers,
				SessionEndingLogEntry::CommitContract,
				SessionEndingLogEntry::CommitTotalValidatorStakes,
				SessionEndingLogEntry::UnlockCurrency,
				SessionEndingLogEntry::UnlockDelegatorNfts,
			]
		);
	});
}

#[rstest]
fn session_ending_is_resumable(mut ext: TestExternalities) {
	ext.execute_with(|| {
		setup_session_ending();

		// stop the session ending after calculating the total stake (on_idle(1) + total_stake(4000))
		AllPalletsWithSystem::on_idle(42, Weight::from_all(4001));

		assert!(matches!(
			&SessionEndings::<Test>::get()[0],
			SessionEnding {
				state: State::Reward {
					sweep: reward::Sweep::IterValidators {
						reward_state: reward::RewardValidator::MaybeResetValidatorState,
						context: reward::ValidatorContext {
							validator: REWARDED_ACC,
							total_committed_stake: 200,
							session_reward: 1000,
							total_contribution: 0
						}
					},
					..
				},
				..
			}
		));

		next_session_without_on_idle();

		// Other session endings are stacking up...
		assert_eq!(SessionEndings::<Test>::get().len(), 2);

		// stop after trying to reset validator state (before rewarding contract) (on_idle(1) + reset_state(1))
		AllPalletsWithSystem::on_idle(42, Weight::from_all(2));

		assert!(matches!(
			&SessionEndings::<Test>::get()[0],
			SessionEnding {
				state: State::Reward {
					sweep: reward::Sweep::IterValidators {
						reward_state: reward::RewardValidator::IterContracts {
							prev_delegator: None,
							total_v_imbalance: 0
						},
						..
					},
					..
				},
				..
			}
		));

		next_session_without_on_idle();

		// Other session endings are stacking up...
		assert_eq!(SessionEndings::<Test>::get().len(), 3);

		// finish the job
		AllPalletsWithSystem::on_idle(42, Weight::MAX);
		assert!(SessionEndings::<Test>::get().is_empty());
	});
}

#[rstest]
fn staging_layer_rotation_works_with_multiple_session_endings_queued(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		const DELEGATION: Balance = 10;

		let delegate = || {
			assert_ok!(Staking::delegate_currency(
				delegator.origin.clone(),
				DELEGATION,
				validator.account_id,
				MinimumStakingPeriod::get().into(),
				MinimumCommission::get(),
			));
		};

		let assert_staged = |layer: StagingLayer, amount: Option<Balance>| {
			assert_eq!(
				Contracts::<Test>::get((
					StorageLayer::Staged(layer),
					validator.account_id,
					delegator.account_id
				))
				.map(|c| c.stake.currency),
				amount
			);
		};

		let assert_committed = |amount: Option<Balance>| {
			assert_eq!(
				Contracts::<Test>::get((
					StorageLayer::Committed,
					validator.account_id,
					delegator.account_id
				))
				.map(|c| c.stake.currency),
				amount
			);
		};

		// To begin with: no in-progress session ending and A is the stagin layer
		assert_eq!(SessionEndings::<Test>::get().len(), 0);
		assert_eq!(CurrentStagingLayer::<Test>::get(), StagingLayer::A);

		delegate();
		next_session_without_on_idle();

		// 1 in-progress session ending and A is still the active stagin layer
		assert_eq!(SessionEndings::<Test>::get().len(), 1);
		assert_eq!(CurrentStagingLayer::<Test>::get(), StagingLayer::A);

		assert_committed(None);
		assert_staged(StagingLayer::A, Some(DELEGATION));

		delegate();
		next_session_without_on_idle();

		// 2 in-progress session ending and A is still the active stagin layer
		assert_eq!(SessionEndings::<Test>::get().len(), 2);
		assert_eq!(CurrentStagingLayer::<Test>::get(), StagingLayer::A);

		assert_committed(None);
		assert_staged(StagingLayer::A, Some(2 * DELEGATION));

		// Processing first session-ending -> committing and rotating stagin layer A
		AllPalletsWithSystem::on_idle(42, Weight::from_all(4024));

		assert_eq!(SessionEndings::<Test>::get().len(), 1);
		assert_eq!(CurrentStagingLayer::<Test>::get(), StagingLayer::B);

		assert_committed(Some(2 * DELEGATION));
		assert_staged(StagingLayer::A, None);
		assert_staged(StagingLayer::B, None);

		delegate();
		next_session_without_on_idle();

		// 2 in-progress session endings with B remaining the active staging layer
		assert_eq!(SessionEndings::<Test>::get().len(), 2);
		assert_eq!(CurrentStagingLayer::<Test>::get(), StagingLayer::B);

		assert_committed(Some(2 * DELEGATION));
		// 2(committed) + 1(delegated)
		assert_staged(StagingLayer::B, Some(3 * DELEGATION));
	});
}
