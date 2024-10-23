// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # I'm online Pallet
//!
//! If the local node is a validator (i.e. contains an authority key), this pallet
//! gossips a heartbeat transaction with each new session. The heartbeat functions
//! as a simple mechanism to signal that the node is online in the current era.
//!
//! Received heartbeats are tracked for one era and reset with each new era. The
//! pallet exposes two public functions to query if a heartbeat has been received
//! in the current era or session.
//!
//! The heartbeat is a signed transaction, which was signed using the session key
//! and includes the recent best block number of the local validators chain.
//! It is submitted as an Unsigned Transaction via off-chain workers.
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//!
//! ## Interface
//!
//! ### Public Functions
//!
//! - `is_online` - True if the validator sent a heartbeat in the current session.
//!
//! ## Usage
//!
//! ```
//! use pallet_im_online::{self as im_online};
//!
//! #[frame_support::pallet]
//! pub mod pallet {
//!     use super::*;
//!     use frame_support::pallet_prelude::*;
//!     use frame_system::pallet_prelude::*;
//!
//!     #[pallet::pallet]
//!     pub struct Pallet<T>(_);
//!
//!     #[pallet::config]
//!     pub trait Config: frame_system::Config + im_online::Config {}
//!
//!     #[pallet::call]
//!     impl<T: Config> Pallet<T> {
//!         #[pallet::weight(0)]
//!         pub fn is_online(origin: OriginFor<T>, authority_index: u32) -> DispatchResult {
//!             let _sender = ensure_signed(origin)?;
//!             let _is_online = <im_online::Pallet<T>>::is_online(authority_index);
//!             Ok(())
//!         }
//!     }
//! }
//! # fn main() { }
//! ```
//!
//! ## Dependencies
//!
//! This pallet depends on the [Session pallet](../pallet_session/index.html).

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
// Expect lints caused by procmacros
#![expect(clippy::manual_inspect)]

// mod benchmarking;
// pub mod migration;
// mod mock;
// mod tests;
// pub mod weights;

use sdk::{
	frame_support, frame_system, pallet_authorship, pallet_session, sp_application_crypto, sp_io,
	sp_runtime, sp_staking, sp_std,
};

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	pallet_prelude::*,
	traits::{
		EstimateNextSessionRotation, Get, OneSessionHandler, ValidatorSet,
		ValidatorSetWithIdentification,
	},
};
use frame_system::{
	offchain::{SendTransactionTypes, SubmitTransaction},
	pallet_prelude::*,
};
pub use pallet::*;
use scale_info::TypeInfo;
use sp_application_crypto::RuntimeAppPublic;
use sp_runtime::{
	offchain::storage::{MutateStorageError, StorageRetrievalError, StorageValueRef},
	traits::{AtLeast32BitUnsigned, Convert, OpaqueKeys, Saturating, TrailingZeroInput},
	PerThing, Perbill, Permill, RuntimeDebug, SaturatedConversion,
};
use sp_staking::{
	offence::{Kind, Offence, ReportOffence},
	SessionIndex,
};

use utils::storage::{ClearAll, ClearAllPrefix};

use sp_std::prelude::*;
// pub use weights::WeightInfo;

pub mod sr25519 {
	mod app_sr25519 {
		use sdk::sp_application_crypto;
		use sp_application_crypto::{app_crypto, key_types::IM_ONLINE, sr25519};
		app_crypto!(sr25519, IM_ONLINE);
	}

	sdk::sp_application_crypto::with_pair! {
		/// An i'm online keypair using sr25519 as its crypto.
		pub type AuthorityPair = app_sr25519::Pair;
	}

	/// An i'm online signature using sr25519 as its crypto.
	pub type AuthoritySignature = app_sr25519::Signature;

	/// An i'm online identifier using sr25519 as its crypto.
	pub type AuthorityId = app_sr25519::Public;
}

pub mod ed25519 {
	mod app_ed25519 {
		use sdk::sp_application_crypto::{app_crypto, ed25519, key_types::IM_ONLINE};
		app_crypto!(ed25519, IM_ONLINE);
	}

	sdk::sp_application_crypto::with_pair! {
		/// An i'm online keypair using ed25519 as its crypto.
		pub type AuthorityPair = app_ed25519::Pair;
	}

	/// An i'm online signature using ed25519 as its crypto.
	pub type AuthoritySignature = app_ed25519::Signature;

	/// An i'm online identifier using ed25519 as its crypto.
	pub type AuthorityId = app_ed25519::Public;
}

const DB_PREFIX: &[u8] = b"parity/im-online-heartbeat/";
/// How many blocks do we wait for heartbeat transaction to be included
/// before sending another one.
const INCLUDE_THRESHOLD: u32 = 3;

/// Status of the offchain worker code.
///
/// This stores the block number at which heartbeat was requested and when the worker
/// has actually managed to produce it.
/// Note we store such status for every `authority_index` separately.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
struct HeartbeatStatus<BlockNumber> {
	/// An index of the session that we are supposed to send heartbeat for.
	pub session_index: SessionIndex,
	/// A block number at which the heartbeat for that session has been actually sent.
	///
	/// It may be 0 in case the sending failed. In such case we should just retry
	/// as soon as possible (i.e. in a worker running for the next block).
	pub sent_at: BlockNumber,
}

impl<BlockNumber: PartialEq + AtLeast32BitUnsigned + Copy> HeartbeatStatus<BlockNumber> {
	/// Returns true if heartbeat has been recently sent.
	///
	/// Parameters:
	/// `session_index` - index of current session.
	/// `now` - block at which the offchain worker is running.
	///
	/// This function will return `true` iff:
	/// 1. the session index is the same (we don't care if it went up or down)
	/// 2. the heartbeat has been sent recently (within the threshold)
	///
	/// The reasoning for 1. is that it's better to send an extra heartbeat than
	/// to stall or not send one in case of a bug.
	fn is_recent(&self, session_index: SessionIndex, now: BlockNumber) -> bool {
		self.session_index == session_index && self.sent_at + INCLUDE_THRESHOLD.into() > now
	}
}

/// Error which may occur while executing the off-chain code.
#[cfg_attr(test, derive(PartialEq))]
enum OffchainErr<BlockNumber> {
	TooEarly,
	WaitingForInclusion(BlockNumber),
	AlreadyOnline,
	FailedSigning,
	FailedToAcquireLock,
	SubmitTransaction,
}

impl<BlockNumber: sp_std::fmt::Debug> sp_std::fmt::Debug for OffchainErr<BlockNumber> {
	fn fmt(&self, fmt: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		match *self {
			OffchainErr::TooEarly => write!(fmt, "Too early to send heartbeat."),
			OffchainErr::WaitingForInclusion(ref block) => {
				write!(fmt, "Heartbeat already sent at {block:?}. Waiting for inclusion.")
			},
			OffchainErr::AlreadyOnline => {
				write!(fmt, "Authority is already online")
			},
			OffchainErr::FailedSigning => write!(fmt, "Failed to sign heartbeat"),
			OffchainErr::FailedToAcquireLock => write!(fmt, "Failed to acquire lock"),
			OffchainErr::SubmitTransaction => write!(fmt, "Failed to submit transaction"),
		}
	}
}

/// Heartbeat which is sent/received.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct Heartbeat<BlockNumber, AuthorityKey>
where
	BlockNumber: PartialEq + Eq + Decode + Encode,
{
	/// Block number at the time heartbeat is created..
	pub block_number: BlockNumber,
	/// Index of the current session.
	pub session_index: SessionIndex,
	/// An index of the authority on the list of validators.
	pub key: AuthorityKey,
}

/// A type for representing the validator id in a session.
pub type ValidatorId<T> = <T as pallet_session::Config>::ValidatorId;

/// A tuple of (ValidatorId, Identification) where `Identification` is the full identification of
/// `ValidatorId`.
pub type IdentificationTuple<T> = (
	ValidatorId<T>,
	<<T as Config>::ValidatorSet as ValidatorSetWithIdentification<
		<T as frame_system::Config>::AccountId,
	>>::Identification,
);

type OffchainResult<T, A> = Result<A, OffchainErr<BlockNumberFor<T>>>;

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;

	/// The in-code storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		SendTransactionTypes<Call<Self>> + sdk::frame_system::Config + pallet_session::Config
	{
		/// The identifier type for an authority.
		type AuthorityKey: Member
			+ Parameter
			+ RuntimeAppPublic
			+ Ord
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen;

		/// The maximum number of peers to be stored in `ReceivedHeartbeats`
		type MaxPeerInHeartbeats: Get<u32>;

		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as sdk::frame_system::Config>::RuntimeEvent>;

		/// A type for retrieving the validators supposed to be online in a session.
		type ValidatorSet: ValidatorSetWithIdentification<
			Self::AccountId,
			ValidatorId = Self::ValidatorId,
		>;

		/// A type that gives us the ability to submit unresponsiveness offence reports.
		type ReportUnresponsiveness: ReportOffence<
			Self::AccountId,
			IdentificationTuple<Self>,
			UnresponsivenessOffence<IdentificationTuple<Self>>,
		>;

		/// A configuration for base priority of unsigned transactions.
		///
		/// This is exposed so that it can be tuned for particular runtime, when
		/// multiple pallets send unsigned transactions.
		#[pallet::constant]
		type UnsignedPriority: Get<TransactionPriority>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo; //: WeightInfo
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new heartbeat was received from `AuthorityId`.
		HeartbeatReceived { key: T::AuthorityKey },
		/// At the end of the session, no offence was committed.
		AllGood,
		/// At the end of the session, at least one validator was found to be offline.
		SomeOffline { offline: Vec<IdentificationTuple<T>> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Non existent public key.
		InvalidKey,
		/// Duplicated heartbeat.
		DuplicatedHeartbeat,
	}

	/// The block number after which it's ok to send heartbeats in the current
	/// session.
	///
	/// At the beginning of each session we set this to a value that should fall
	/// roughly in the middle of the session duration. The idea is to first wait for
	/// the validators to produce a block in the current session, so that the
	/// heartbeat later on will not be necessary.
	///
	/// This value will only be used as a fallback if we fail to get a proper session
	/// progress estimate from `NextSessionRotation`, as those estimates should be
	/// more accurate then the value we calculate for `HeartbeatAfter`.
	#[pallet::storage]
	pub type HeartbeatAfter<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	/// The current set of keys that may issue a heartbeat.
	#[pallet::storage]
	pub type Keys<T: Config> =
		StorageMap<_, Twox64Concat, T::AuthorityKey, ValidatorId<T>, OptionQuery>;

	/// For each session index, we keep a mapping of `SessionIndex` and `ValidatorId`.
	#[pallet::storage]
	pub type ReceivedHeartbeats<T: Config> =
		StorageDoubleMap<_, Twox64Concat, SessionIndex, Twox64Concat, ValidatorId<T>, bool>;

	/// For each session index, we keep a mapping of `ValidatorId<T>` to the
	/// number of blocks authored by the given authority.
	#[pallet::storage]
	pub type AuthoredBlocks<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		SessionIndex,
		Twox64Concat,
		ValidatorId<T>,
		u32,
		ValueQuery,
	>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// ## Complexity:
		/// - `O(K)` where K is length of `Keys` (heartbeat.validators_len)
		///   - `O(K)`: decoding of length `K`
		// NOTE: the weight includes the cost of validate_unsigned as it is part of the cost to
		// import block with such an extrinsic.
		#[pallet::call_index(0)]
		// #[pallet::weight(<T as Config>::WeightInfo::validate_unsigned_and_then_heartbeat(
		// 	heartbeat.validators_len,
		// ))]
		pub fn heartbeat(
			origin: OriginFor<T>,
			heartbeat: Heartbeat<BlockNumberFor<T>, T::AuthorityKey>,
			// since signature verification is done in `validate_unsigned`
			// we can skip doing it here again.
			_signature: <T::AuthorityKey as RuntimeAppPublic>::Signature,
		) -> DispatchResult {
			ensure_none(origin)?;

			let current_session = T::ValidatorSet::session_index();

			let Some(validator) = Keys::<T>::get(&heartbeat.key) else {
				return Err(Error::<T>::InvalidKey.into());
			};

			if ReceivedHeartbeats::<T>::contains_key(current_session, &validator) {
				return Err(Error::<T>::DuplicatedHeartbeat.into());
			}

			Self::deposit_event(Event::<T>::HeartbeatReceived { key: heartbeat.key });
			ReceivedHeartbeats::<T>::insert(current_session, validator, true);

			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(now: BlockNumberFor<T>) {
			// Only send messages if we are a potential validator.
			if sp_io::offchain::is_validator() {
				for res in Self::send_heartbeats(now).into_iter().flatten() {
					if let Err(e) = res {
						log::debug!(
							target: "runtime::im-online",
							"Skipping heartbeat at {:?}: {:?}",
							now,
							e,
						);
					}
				}
			} else {
				log::trace!(
					target: "runtime::im-online",
					"Skipping heartbeat at {:?}. Not a validator.",
					now,
				);
			}
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			if let Call::heartbeat { heartbeat, signature } = call {
				if <Pallet<T>>::is_online(&heartbeat.key) {
					// we already received a heartbeat for this authority
					return InvalidTransaction::Stale.into();
				}

				// check if session index from heartbeat is recent
				let current_session = T::ValidatorSet::session_index();
				if heartbeat.session_index != current_session {
					return InvalidTransaction::Stale.into();
				}

				// check signature (this is expensive so we do it last).
				let signature_valid = heartbeat.using_encoded(|encoded_heartbeat| {
					heartbeat.key.verify(&encoded_heartbeat, signature)
				});

				if !signature_valid {
					return InvalidTransaction::BadProof.into();
				}

				ValidTransaction::with_tag_prefix("ImOnline")
					.priority(T::UnsignedPriority::get())
					.and_provides((current_session, heartbeat.key.clone()))
					.longevity(
						TryInto::<u64>::try_into(
							T::NextSessionRotation::average_session_length() / 2u32.into(),
						)
						.unwrap_or(64_u64),
					)
					.propagate(true)
					.build()
			} else {
				InvalidTransaction::Call.into()
			}
		}
	}
}

/// Keep track of number of authored blocks per authority, uncles are counted as
/// well since they're a valid proof of being online.
impl<T: Config + pallet_authorship::Config>
	pallet_authorship::EventHandler<ValidatorId<T>, BlockNumberFor<T>> for Pallet<T>
{
	fn note_author(author: ValidatorId<T>) {
		Self::note_authorship(author);
	}
}

impl<T: Config> Pallet<T> {
	/// Returns `true` if a heartbeat has been received for the authority at
	/// `authority_index` in the authorities series or if the authority has
	/// authored at least one block, during the current session. Otherwise
	/// `false`.
	pub fn is_online(key: &T::AuthorityKey) -> bool {
		let Some(authority) = Keys::<T>::get(key) else {
			return false;
		};

		Self::is_online_aux(&authority)
	}

	fn is_online_aux(authority: &ValidatorId<T>) -> bool {
		let current_session = T::ValidatorSet::session_index();

		ReceivedHeartbeats::<T>::contains_key(current_session, authority)
			|| AuthoredBlocks::<T>::get(current_session, authority) != 0
	}

	/// Returns `true` if a heartbeat has been received for the authority at `authority_index` in
	/// the authorities series, during the current session. Otherwise `false`.
	pub fn received_heartbeat_in_current_session(key: T::AuthorityKey) -> bool {
		let current_session = T::ValidatorSet::session_index();

		let Some(authority) = Keys::<T>::get(key) else {
			return false;
		};

		ReceivedHeartbeats::<T>::contains_key(current_session, authority)
	}

	/// Note that the given authority has authored a block in the current session.
	fn note_authorship(author: ValidatorId<T>) {
		let current_session = T::ValidatorSet::session_index();
		AuthoredBlocks::<T>::mutate(current_session, author, |authored: &mut u32| *authored += 1);
	}

	pub(crate) fn send_heartbeats(
		block_number: BlockNumberFor<T>,
	) -> OffchainResult<T, impl Iterator<Item = OffchainResult<T, ()>>> {
		const START_HEARTBEAT_RANDOM_PERIOD: Permill = Permill::from_percent(10);
		const START_HEARTBEAT_FINAL_PERIOD: Permill = Permill::from_percent(80);

		// this should give us a residual probability of 1/SESSION_LENGTH of sending an heartbeat,
		// i.e. all heartbeats spread uniformly, over most of the session. as the session progresses
		// the probability of sending an heartbeat starts to increase exponentially.
		let random_choice = |progress: Permill| {
			// given session progress `p` and session length `l`
			// the threshold formula is: p^6 + 1/l
			let session_length = T::NextSessionRotation::average_session_length();
			let residual = Permill::from_rational(1u32, session_length.saturated_into());
			let threshold: Permill = progress.saturating_pow(6).saturating_add(residual);

			let seed = sp_io::offchain::random_seed();
			let random = <u32>::decode(&mut TrailingZeroInput::new(seed.as_ref()))
				.expect("input is padded with zeroes; qed");
			let random = Permill::from_parts(random % Permill::ACCURACY);

			random <= threshold
		};

		let should_heartbeat = if let (Some(progress), _) =
			T::NextSessionRotation::estimate_current_session_progress(block_number)
		{
			// we try to get an estimate of the current session progress first since it should
			// provide more accurate results. we will start an early heartbeat period where we'll
			// randomly pick whether to heartbeat. after 80% of the session has elapsed, if we
			// haven't sent an heartbeat yet we'll send one unconditionally. the idea is to prevent
			// all nodes from sending the heartbeats at the same block and causing a temporary (but
			// deterministic) spike in transactions.
			progress >= START_HEARTBEAT_FINAL_PERIOD
				|| progress >= START_HEARTBEAT_RANDOM_PERIOD && random_choice(progress)
		} else {
			// otherwise we fallback to using the block number calculated at the beginning
			// of the session that should roughly correspond to the middle of the session
			let heartbeat_after = <HeartbeatAfter<T>>::get();
			block_number >= heartbeat_after
		};

		if !should_heartbeat {
			return Err(OffchainErr::TooEarly);
		}

		let session_index = T::ValidatorSet::session_index();

		Ok(Self::local_authority_keys()
			.map(move |key| Self::send_single_heartbeat(&key, session_index, block_number)))
	}

	fn send_single_heartbeat(
		key: &T::AuthorityKey,
		session_index: SessionIndex,
		block_number: BlockNumberFor<T>,
	) -> OffchainResult<T, ()> {
		// A helper function to prepare heartbeat call.
		let prepare_heartbeat = || -> OffchainResult<T, Call<T>> {
			let heartbeat = Heartbeat { block_number, session_index, key: key.clone() };

			let signature = key.sign(&heartbeat.encode()).ok_or(OffchainErr::FailedSigning)?;

			Ok(Call::heartbeat { heartbeat, signature })
		};

		if Self::is_online(key) {
			return Err(OffchainErr::AlreadyOnline);
		}

		// acquire lock for that authority at current heartbeat to make sure we don't
		// send concurrent heartbeats.
		Self::with_heartbeat_lock(key, session_index, block_number, || {
			let call = prepare_heartbeat()?;
			log::info!(
				target: "runtime::im-online",
				"Reporting im-online at block: {:?} (session: {:?}): {:?}",
				block_number,
				session_index,
				call,
			);

			SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
				.map_err(|()| OffchainErr::SubmitTransaction)?;

			Ok(())
		})
	}

	fn local_authority_keys() -> impl Iterator<Item = T::AuthorityKey> {
		// on-chain storage
		//
		// At index `idx`:
		// 1. A (ImOnline) public key to be used by a validator at index `idx` to send im-online
		//    heartbeats.
		let authorities = Keys::<T>::iter_keys();

		// local keystore
		//
		// All `ImOnline` public (+private) keys currently in the local keystore.
		let mut local_keys = T::AuthorityKey::all();

		local_keys.sort();

		authorities.into_iter().filter_map(move |authority| {
			local_keys
				.binary_search(&authority)
				.ok()
				.map(|location| local_keys[location].clone())
		})
	}

	fn with_heartbeat_lock<R>(
		key: &T::AuthorityKey,
		session_index: SessionIndex,
		now: BlockNumberFor<T>,
		f: impl FnOnce() -> OffchainResult<T, R>,
	) -> OffchainResult<T, R> {
		let storage_key = {
			let mut storage_key = DB_PREFIX.to_vec();
			storage_key.extend(key.encode());
			storage_key
		};

		let storage = StorageValueRef::persistent(&storage_key);
		let res = storage.mutate(
			|status: Result<Option<HeartbeatStatus<BlockNumberFor<T>>>, StorageRetrievalError>| {
				// Check if there is already a lock for that particular block.
				// This means that the heartbeat has already been sent, and we are just waiting
				// for it to be included. However if it doesn't get included for INCLUDE_THRESHOLD
				// we will re-send it.
				match status {
					// we are still waiting for inclusion.
					Ok(Some(status)) if status.is_recent(session_index, now) => {
						Err(OffchainErr::WaitingForInclusion(status.sent_at))
					},
					// attempt to set new status
					_ => Ok(HeartbeatStatus { session_index, sent_at: now }),
				}
			},
		);
		if let Err(MutateStorageError::ValueFunctionFailed(err)) = res {
			return Err(err);
		}

		let mut new_status = res.map_err(|_| OffchainErr::FailedToAcquireLock)?;

		// we got the lock, let's try to send the heartbeat.
		let res = f();

		// clear the lock in case we have failed to send transaction.
		if res.is_err() {
			new_status.sent_at = 0u32.into();
			storage.set(&new_status);
		}

		res
	}

	fn rotate_keys() {
		Keys::<T>::clear_all(350);

		//TODO: when should ValidatorSet::validators() be called?

		for (v, k) in T::ValidatorSet::validators()
			.into_iter()
			.filter_map(|v| pallet_session::NextKeys::<T>::get(&v).map(|a| (v, a)))
			.filter_map(|(k, v)| v.get::<T::AuthorityKey>(T::AuthorityKey::ID).map(|a| (k, a)))
		{
			Keys::<T>::insert(k, v);
		}
	}
}

impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T> {
	type Public = T::AuthorityKey;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
	type Key = T::AuthorityKey;

	fn on_genesis_session<'a, I: 'a>(_validators: I) {
		Self::rotate_keys();
	}

	fn on_new_session<'a, I: 'a>(_changed: bool, _validators: I, _queued_validators: I) {
		// Tell the offchain worker to start making the next session's heartbeats.
		// Since we consider producing blocks as being online,
		// the heartbeat is deferred a bit to prevent spamming.
		let block_number = <frame_system::Pallet<T>>::block_number();
		let half_session = T::NextSessionRotation::average_session_length() / 2u32.into();

		<HeartbeatAfter<T>>::put(block_number + half_session);

		Self::rotate_keys();
	}

	fn on_before_session_ending() {
		let session_index = T::ValidatorSet::session_index();
		let validators = T::ValidatorSet::validators();
		let validator_set_count = validators.len() as u32;

		let offenders = validators
			.into_iter()
			.filter(|id| !Self::is_online_aux(id))
			.filter_map(|id| {
				<T::ValidatorSet as ValidatorSetWithIdentification<T::AccountId>>::IdentificationOf::convert(
					id.clone()
				).map(|full_id| (id, full_id))
			})
			.collect::<Vec<IdentificationTuple<T>>>();

		// Remove all received heartbeats and number of authored blocks from the
		// current session, they have already been processed and won't be needed
		// anymore.
		ReceivedHeartbeats::<T>::clear_all_prefix(&session_index, 350);
		AuthoredBlocks::<T>::clear_all_prefix(&session_index, 350);

		if offenders.is_empty() {
			Self::deposit_event(Event::<T>::AllGood);
		} else {
			Self::deposit_event(Event::<T>::SomeOffline { offline: offenders.clone() });

			let offence = UnresponsivenessOffence { session_index, validator_set_count, offenders };
			if let Err(e) = T::ReportUnresponsiveness::report_offence(vec![], offence) {
				sp_runtime::print(e);
			}
		}
	}

	fn on_disabled(_i: u32) {
		// ignore
	}
}

/// An offence that is filed if a validator didn't send a heartbeat message.
#[derive(RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Clone, PartialEq, Eq))]
pub struct UnresponsivenessOffence<Offender> {
	/// The current session index in which we report the unresponsive validators.
	///
	/// It acts as a time measure for unresponsiveness reports and effectively will always point
	/// at the end of the session.
	pub session_index: SessionIndex,
	/// The size of the validator set in current session/era.
	pub validator_set_count: u32,
	/// Authorities that were unresponsive during the current era.
	pub offenders: Vec<Offender>,
}

impl<Offender: Clone> Offence<Offender> for UnresponsivenessOffence<Offender> {
	const ID: Kind = *b"im-online:offlin";
	type TimeSlot = SessionIndex;

	fn offenders(&self) -> Vec<Offender> {
		self.offenders.clone()
	}

	fn session_index(&self) -> SessionIndex {
		self.session_index
	}

	fn validator_set_count(&self) -> u32 {
		self.validator_set_count
	}

	fn time_slot(&self) -> Self::TimeSlot {
		self.session_index
	}

	fn slash_fraction(&self, offenders: u32) -> Perbill {
		// the formula is min((3 * (k - (n / 10 + 1))) / n, 1) * 0.07
		// basically, 10% can be offline with no slash, but after that, it linearly climbs up to 7%
		// when 13/30 are offline (around 5% when 1/3 are offline).
		if let Some(threshold) = offenders.checked_sub(self.validator_set_count / 10 + 1) {
			let x = Perbill::from_rational(3 * threshold, self.validator_set_count);
			x.saturating_mul(Perbill::from_percent(7))
		} else {
			Perbill::default()
		}
	}
}
