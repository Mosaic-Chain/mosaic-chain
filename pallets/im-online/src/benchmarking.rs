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

//! I'm Online pallet benchmarking.

use super::*;

use sdk::{frame_benchmarking, frame_support, frame_system, sp_core, sp_runtime};

use frame_benchmarking::v2::*;
use frame_support::traits::UnfilteredDispatchable;
use frame_system::RawOrigin;
use sp_core::crypto::DEV_PHRASE;
use sp_runtime::{
	traits::{ValidateUnsigned, Zero},
	transaction_validity::TransactionSource,
};

use crate::Pallet as ImOnline;

const MAX_KEYS: u32 = 100;

pub fn create_heartbeat<T: Config>(
	k: u32,
) -> Result<(HeartbeatOf<T>, <T::AuthorityKey as RuntimeAppPublic>::Signature), &'static str> {
	let alice_auth_key = T::AuthorityKey::generate_pair(Some(DEV_PHRASE.into()));

	Keys::<T>::insert(&alice_auth_key, account::<ValidatorId<T>>("alice", 0, 0));

	for i in 1..k {
		let key = T::AuthorityKey::generate_pair(None);
		let acc: ValidatorId<T> = account("validator", i, 0);
		Keys::<T>::insert(key, acc);
	}

	let input_heartbeat = Heartbeat {
		block_number: frame_system::pallet_prelude::BlockNumberFor::<T>::zero(),
		session_index: 0,
		key: alice_auth_key,
	};

	let encoded_heartbeat = input_heartbeat.encode();
	let signature =
		input_heartbeat.key.sign(&encoded_heartbeat).ok_or("couldn't make signature")?;

	Ok((input_heartbeat, signature))
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark(extra)]
	fn heartbeat(k: Linear<1, { MAX_KEYS }>) {
		let (input_heartbeat, signature) = create_heartbeat::<T>(k).unwrap();

		#[extrinsic_call]
		ImOnline::<T>::heartbeat(RawOrigin::None, input_heartbeat, signature);
	}
	#[benchmark(extra)]
	fn validate_unsigned(k: Linear<1, { MAX_KEYS }>) {
		let (input_heartbeat, signature) = create_heartbeat::<T>(k).unwrap();
		let call = Call::heartbeat { heartbeat: input_heartbeat, signature };

		#[block]
		{
			ImOnline::<T>::validate_unsigned(TransactionSource::InBlock, &call)
				.map_err(<&str>::from)
				.unwrap();
		}
	}

	#[benchmark]
	fn validate_unsigned_and_then_heartbeat(k: Linear<1, { MAX_KEYS }>) {
		let (input_heartbeat, signature) = create_heartbeat::<T>(k).unwrap();
		let call = Call::heartbeat { heartbeat: input_heartbeat, signature };
		let call_enc = call.encode();

		#[block]
		{
			ImOnline::<T>::validate_unsigned(TransactionSource::InBlock, &call)
				.map_err(<&str>::from)
				.unwrap();
			<Call<T> as Decode>::decode(&mut &*call_enc)
				.expect("call is encoded above, encoding must be correct")
				.dispatch_bypass_filter(RawOrigin::None.into())
				.unwrap();
		}
	}

	// impl_benchmark_test_suite!(ImOnline, crate::mock::new_test_ext(), crate::mock::Runtime);
}
