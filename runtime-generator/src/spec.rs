use sdk::sc_chain_spec::ChainSpec;

use crate::runtime_builder::RuntimeBuilder;

pub mod common;

#[cfg(feature = "para-chain")]
pub mod para_chain;

#[cfg(feature = "solo-chain")]
pub mod solo_chain;

pub const PROFILES: inventory::iter<Profile> = inventory::iter::<Profile>;

pub type CreateFn = fn(&dyn RuntimeBuilder) -> anyhow::Result<Box<dyn ChainSpec>>;

pub struct Profile {
	pub name: &'static str,
	pub create_fn: CreateFn,
}

impl Profile {
	pub const fn new(name: &'static str, create_fn: CreateFn) -> Self {
		Self { name, create_fn }
	}
}

inventory::collect!(Profile);
