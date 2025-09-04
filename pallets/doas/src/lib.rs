#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

use sdk::sp_runtime::traits::StaticLookup;
use sdk::sp_std::prelude::*;

pub use pallet::*;
pub use weights::WeightInfo;

type AccountIdLookupOf<T> = <<T as sdk::frame_system::Config>::Lookup as StaticLookup>::Source;

#[expect(
	clippy::useless_conversion,
	reason = "pallet macro tries to convert PostDispatchInfo to itself"
)]
#[sdk::frame_support::pallet]
pub mod pallet {
	use super::*;
	use sdk::frame_support::{
		dispatch::GetDispatchInfo,
		pallet_prelude::*,
		traits::{EnsureOrigin, UnfilteredDispatchable},
	};
	use sdk::frame_system::{pallet_prelude::*, RawOrigin};

	#[pallet::config]
	pub trait Config: sdk::frame_system::Config {
		/// Because this pallet can do any calls, it depends on the runtime's definition of a call.
		type RuntimeCall: Parameter
			+ UnfilteredDispatchable<RuntimeOrigin = Self::RuntimeOrigin>
			+ GetDispatchInfo;

		/// The origin that can do calls with this pallet (
		/// pallet_collective::EnsureProportionAtLeast is a good choice)
		type EnsureOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Type representing the weights of calls in this pallet
		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Calls into another pallet as the root origin if the provided origin is
		/// successful or root.
		#[pallet::call_index(0)]
		#[pallet::weight({
			let dispatch_info = call.get_dispatch_info();
			(
				T::WeightInfo::doas_root().saturating_add(dispatch_info.call_weight),
				dispatch_info.class
			)
		})]
		pub fn doas_root(
			origin: OriginFor<T>,
			call: Box<<T as Config>::RuntimeCall>,
		) -> DispatchResultWithPostInfo {
			T::EnsureOrigin::ensure_origin_or_root(origin)?;

			let res = call.dispatch_bypass_filter(RawOrigin::Root.into());
			Self::deposit_event(Event::DidAsRoot {
				doas_result: res.map(|_| ()).map_err(|e| e.error),
			});

			Ok(Pays::No.into())
		}

		/// Calls into another pallet as the root origin if the provided origin is
		/// successful or root. The weight of the call is provided by the caller,
		/// which is suitable for runtime upgrades or other huge calls.
		#[pallet::call_index(1)]
		#[pallet::weight((*weight, call.get_dispatch_info().class))]
		pub fn doas_root_unchecked_weight(
			origin: OriginFor<T>,
			call: Box<<T as Config>::RuntimeCall>,
			weight: Weight,
		) -> DispatchResultWithPostInfo {
			T::EnsureOrigin::ensure_origin_or_root(origin)?;

			let _ = weight;

			let res = call.dispatch_bypass_filter(RawOrigin::Root.into());
			Self::deposit_event(Event::DidAsRoot {
				doas_result: res.map(|_| ()).map_err(|e| e.error),
			});

			Ok(Pays::No.into())
		}

		/// Calls into another pallet as a given account if the provided origin is
		/// successful or root.
		#[pallet::call_index(2)]
		#[pallet::weight({
			let dispatch_info = call.get_dispatch_info();
			(
				T::WeightInfo::doas().saturating_add(dispatch_info.call_weight),
				dispatch_info.class
			)
		})]
		pub fn doas(
			origin: OriginFor<T>,
			who: AccountIdLookupOf<T>,
			call: Box<<T as Config>::RuntimeCall>,
		) -> DispatchResultWithPostInfo {
			T::EnsureOrigin::ensure_origin_or_root(origin)?;

			let who = T::Lookup::lookup(who)?;
			let res = call.dispatch_bypass_filter(RawOrigin::Signed(who).into());
			Self::deposit_event(Event::DidAs { doas_result: res.map(|_| ()).map_err(|e| e.error) });

			Ok(Pays::No.into())
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		DidAsRoot { doas_result: DispatchResult },
		DidAs { doas_result: DispatchResult },
	}
}
