use frame_support::pallet_prelude::DispatchResult;
pub use sp_staking::SessionIndex;

#[allow(unused)]
pub trait SessionHook {
	/// Called on genesis session
	fn session_genesis(idx: SessionIndex) -> DispatchResult {
		Ok(())
	}

	/// Called when a session is planned
	fn session_planned(idx: SessionIndex) -> DispatchResult {
		Ok(())
	}

	/// Called when a session is started
	fn session_started(idx: SessionIndex) -> DispatchResult {
		Ok(())
	}

	/// Called before session ends
	fn session_ended(idx: SessionIndex) -> DispatchResult {
		Ok(())
	}
}

#[impl_trait_for_tuples::impl_for_tuples(16)]
impl SessionHook for Tuple {
	fn session_genesis(idx: SessionIndex) -> DispatchResult {
		for_tuples!( #( Tuple::session_genesis(idx)?; )* );
		Ok(())
	}

	fn session_planned(idx: SessionIndex) -> DispatchResult {
		for_tuples!( #( Tuple::session_planned(idx)?; )* );
		Ok(())
	}

	fn session_started(idx: SessionIndex) -> DispatchResult {
		for_tuples!( #( Tuple::session_started(idx)?; )* );
		Ok(())
	}

	fn session_ended(idx: SessionIndex) -> DispatchResult {
		for_tuples!( #( Tuple::session_ended(idx)?; )* );
		Ok(())
	}
}
