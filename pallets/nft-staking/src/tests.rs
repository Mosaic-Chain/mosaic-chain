use sdk::{frame_support, frame_system, pallet_balances, sp_io, sp_runtime, sp_staking};

use frame_support::{
	assert_err, assert_noop, assert_ok,
	traits::{Currency as _, ValidatorSet as _},
};
use frame_system::RawOrigin;
use sp_io::TestExternalities;
use sp_runtime::{PerThing, Perbill};
use sp_staking::offence::ReportOffence;

use utils::{
	traits::{NftDelegation, NftStaking},
	SessionIndex,
};

use rstest::{fixture, rstest};
use rstest_reuse::{apply, template};

use crate::{
	assert_current_contract, assert_current_validator_stake, assert_validator_state, mock::*,
	ChillReason, Contract, Contracts, Error, Event, KickReason, PermissionType,
	SelectableValidators, SlashableValidators, Stake, TotalValidatorStake, TotalValidatorStakes,
	ValidatorDetails, ValidatorState, ValidatorStates, Validators,
};

mod test_bases;
use test_bases::*;

mod bind;
mod chill;
mod delegate_currency;
mod delegate_nft;
mod disable_delegations;
mod enable_delgations;
mod kick;
mod self_stake_currency;
mod self_stake_nft;
mod self_unstake_currency;
mod self_unstake_nft;
mod set_commission;
mod set_minimum_staking_period;
mod topup;
mod unbind;
mod unchill;
mod undelegate_currency;
mod undelegate_nft;

mod autochill;
mod delegator_nft_expire;
mod reward;
mod slash;
mod validator_set_impls;

const NOMINAL_VALUE: Balance = 100;

trait BindState {}

struct Bound;

impl BindState for Bound {}

struct Unbound;

impl BindState for Unbound {}

struct Validator<T: BindState> {
	account_id: AccountId,
	origin: RuntimeOrigin,
	permission_nft: u32,
	_state: T,
}

impl Validator<Unbound> {
	pub fn bind(self) -> Validator<Bound> {
		Staking::bind_validator(self.origin.clone(), self.permission_nft).expect("could bind nft");

		Validator {
			account_id: self.account_id,
			origin: self.origin,
			permission_nft: self.permission_nft,
			_state: Bound,
		}
	}
}

impl<T: BindState> Validator<T> {
	/// Simulate an offence
	pub fn offend(&self) {
		let offence =
			Offence { offenders: vec![self.account_id.clone()], session: Session::current_index() };

		Offences::report_offence(vec![account(42)], offence).expect("Could report offence");
	}
}

struct BindParams {
	account_id: AccountId,
	permission: PermissionType,
	nominal_value: Balance,
}

impl Default for BindParams {
	fn default() -> Self {
		Self {
			account_id: account(0),
			permission: PermissionType::DPoS,
			nominal_value: NOMINAL_VALUE,
		}
	}
}

impl BindParams {
	#[allow(unused)]
	pub fn account_id(mut self, account_id: AccountId) -> Self {
		self.account_id = account_id;
		self
	}

	pub fn account_index(mut self, index: u64) -> Self {
		self.account_id = account(index);
		self
	}

	pub fn permission(mut self, permission: PermissionType) -> Self {
		self.permission = permission;
		self
	}

	pub fn nft_nominal_value(mut self, nominal_value: Balance) -> Self {
		self.nominal_value = nominal_value;
		self
	}

	pub fn mint(self) -> Validator<Unbound> {
		let origin = origin(self.account_id.clone());

		let nft = NftStakingHandler::mint(&self.account_id, &self.permission, &self.nominal_value)
			.expect("could mint permission nft");

		Validator { account_id: self.account_id, origin, permission_nft: nft, _state: Unbound }
	}
}

struct EndowedAccount {
	account_id: AccountId,
	origin: RuntimeOrigin,
	delegator_nft: u32,
}

struct EndowParams {
	account_id: AccountId,
	currency: Balance,
	nominal_value: Balance,
	expiry: SessionIndex,
}

impl Default for EndowParams {
	fn default() -> Self {
		Self {
			account_id: account(100),
			currency: 100_000,
			nominal_value: NOMINAL_VALUE,
			expiry: 10_000,
		}
	}
}

impl EndowParams {
	pub fn account_id(mut self, account_id: AccountId) -> Self {
		self.account_id = account_id;
		self
	}

	#[allow(unused)]
	pub fn account_index(mut self, index: u64) -> Self {
		self.account_id = account(index);
		self
	}

	pub fn currency(mut self, currency: Balance) -> Self {
		self.currency = currency;
		self
	}

	pub fn nft_nominal_value(mut self, nominal_value: Balance) -> Self {
		self.nominal_value = nominal_value;
		self
	}

	pub fn nft_expiry(mut self, expiry: SessionIndex) -> Self {
		self.expiry = expiry;
		self
	}

	pub fn endow(self) -> EndowedAccount {
		let origin = origin(self.account_id.clone());

		let _ = Balances::deposit_creating(&self.account_id, self.currency);
		let nft =
			NftDelegationHandler::mint(self.account_id.clone(), self.expiry, self.nominal_value);

		EndowedAccount { account_id: self.account_id, origin, delegator_nft: nft }
	}
}

fn origin(account: AccountId) -> RuntimeOrigin {
	RuntimeOrigin::from(RawOrigin::Signed(account))
}

/// Run to the session after the minimum staking period expires, so that contracts can be unstaked from.
fn skip_min_staking_period() {
	// 1 session = 1 block
	let index = System::block_number() + u64::from(MinimumStakingPeriod::get().get());
	run_to_block(index, |_| {});
}

/// Skip to the next session
fn next_session() {
	// 1 session = 1 block
	run_to_block(System::block_number() + 1, |_| {});
}

#[macro_export]
macro_rules! assert_validator_state {
	($validator:expr, $pattern:pat $(if $guard:expr)?) => {{
				assert!(matches!(ValidatorStates::<Test>::get($validator), $pattern $(if $guard)?))
	}};
}

#[macro_export]
macro_rules! assert_current_contract {
	($validator:expr, $delegator:expr, $pattern:pat $(if $guard:expr)?) => {{
				assert!(matches!(Contracts::<Test>::get($validator, $delegator).current(), $pattern $(if $guard)?))
	}};
}

#[macro_export]
macro_rules! assert_current_validator_stake {
	($validator:expr, $pattern:pat $(if $guard:expr)?) => {{
				assert!(matches!(TotalValidatorStakes::<Test>::get($validator).current(), $pattern $(if $guard)?))
	}};
}
