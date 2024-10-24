use sdk::{sc_chain_spec, sc_service};

use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use serde::{Deserialize, Serialize};

// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<Extensions>;

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
	/// The relay chain of the Parachain.
	pub relay_chain: String,
	/// The id of the Parachain.
	pub para_id: u32,
}

impl Extensions {
	/// Try to get the extension from the given `ChainSpec`.
	pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
		sc_chain_spec::get_extension(chain_spec.extensions())
	}
}

#[cfg(feature = "dev-spec")]
pub fn dev_spec() -> Box<dyn sc_service::ChainSpec> {
	let spec_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/dev-spec.json"));
	Box::new(
		ChainSpec::from_json_bytes(spec_bytes).expect("build script builds the correct chainspec"),
	)
}
