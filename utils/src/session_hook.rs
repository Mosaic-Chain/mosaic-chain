pub use sp_staking::SessionIndex;
pub trait SessionHook {
	/// Called on genesis session
	fn session_genesis(idx: SessionIndex) {}
	/// Called when a session is planned
	fn session_planned(idx: SessionIndex) {}
	/// Called when a session is started
	fn session_started(idx: SessionIndex) {}
	/// Called before session ends
	fn session_ended(idx: SessionIndex) {}
}

#[impl_trait_for_tuples::impl_for_tuples(16)]
impl SessionHook for Tuple {
	fn session_genesis(idx: SessionIndex) {
		for_tuples!( #( Tuple::session_genesis(idx); )* );
	}
	fn session_planned(idx: SessionIndex) {
		for_tuples!( #( Tuple::session_planned(idx); )* );
	}
	fn session_started(idx: SessionIndex) {
		for_tuples!( #( Tuple::session_started(idx); )* );
	}
	fn session_ended(idx: SessionIndex) {
		for_tuples!( #( Tuple::session_ended(idx); )* );
	}
}
