#![cfg_attr(not(feature = "std"), no_std)]

use sdk::sp_runtime::traits::StaticLookup;
use sdk::sp_std::prelude::*;

pub use pallet::*;

type AccountIdLookupOf<T> = <<T as sdk::frame_system::Config>::Lookup as StaticLookup>::Source;

#[sdk::frame_support::pallet(dev_mode)]
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
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as sdk::frame_system::Config>::RuntimeEvent>;

		type RuntimeCall: Parameter
			+ UnfilteredDispatchable<RuntimeOrigin = Self::RuntimeOrigin>
			+ GetDispatchInfo;

		type EnsureOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
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

		#[pallet::call_index(1)]
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

		#[pallet::call_index(2)]
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
