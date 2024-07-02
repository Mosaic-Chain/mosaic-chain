use cumulus_primitives_core::ParaId;
use parachain_template_runtime::{AccountId, AuraId, Signature, EXISTENTIAL_DEPOSIT};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::sr25519;
use sp_runtime::traits::{IdentifyAccount, Verify};

#[cfg(feature = "local")]
use sp_core::{Pair, Public};

#[cfg(not(feature = "local"))]
use sp_core::bytes;

// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<(), Extensions>;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;
#[cfg(feature = "local")]
const PARA_ID: u32 = 2000;

#[cfg(feature = "local")]
const RELAY_CHAIN: &str = "paseo-local";

#[cfg(not(feature = "local"))]
const PARA_ID: u32 = 3377;

#[cfg(not(feature = "local"))]
const RELAY_CHAIN: &str = "polkadot";

/// Helper function to generate a crypto pair from seed
#[cfg(feature = "local")]
pub fn from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{seed}"), None)
		.expect("static values are valid; qed")
		.public()
}

#[cfg(not(feature = "local"))]
pub fn from_hex(hex: &str) -> sr25519::Public {
	let data: [u8; 32] = bytes::from_hex(hex)
		.expect("static values are valid hex")
		.try_into()
		.expect("static value represents a 32 byte long key");

	sr25519::Public::from_raw(data)
}

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

type AccountPublic = <Signature as Verify>::Signer;

/// Generate collator keys from seed.
///
/// This function's return type must always match the session keys of the chain in tuple format.
#[cfg(feature = "local")]
pub fn get_collator_keys_from_seed(seed: &str) -> AuraId {
	from_seed::<AuraId>(seed)
}

/// Helper function to generate an account ID from seed
pub fn get_account_id(pubkey: sr25519::Public) -> AccountId
where
	AccountPublic: From<sr25519::Public>,
{
	AccountPublic::from(pubkey).into_account()
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn session_keys(keys: AuraId) -> parachain_template_runtime::SessionKeys {
	parachain_template_runtime::SessionKeys { aura: keys }
}

#[cfg(feature = "local")]
pub fn development_config() -> ChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "MOS".into());
	properties.insert("tokenDecimals".into(), 18.into());
	properties.insert("ss58Format".into(), 14998.into());

	ChainSpec::builder(
		parachain_template_runtime::WASM_BINARY
			.expect("WASM binary was not built, please build it!"),
		Extensions {
			relay_chain: RELAY_CHAIN.into(),
			// You MUST set this to the correct network!
			para_id: PARA_ID,
		},
	)
	.with_name("Development")
	.with_id("dev")
	.with_chain_type(ChainType::Development)
	.with_genesis_config_patch(testnet_genesis(
		// initial collators.
		vec![
			(
				get_account_id(from_seed::<sr25519::Public>("Alice")),
				get_collator_keys_from_seed("Alice"),
			),
			(
				get_account_id(from_seed::<sr25519::Public>("Bob")),
				get_collator_keys_from_seed("Bob"),
			),
		],
		vec![
			get_account_id(from_seed::<sr25519::Public>("Alice")),
			get_account_id(from_seed::<sr25519::Public>("Bob")),
			get_account_id(from_seed::<sr25519::Public>("Charlie")),
			get_account_id(from_seed::<sr25519::Public>("Dave")),
			get_account_id(from_seed::<sr25519::Public>("Eve")),
			get_account_id(from_seed::<sr25519::Public>("Ferdie")),
			get_account_id(from_seed::<sr25519::Public>("Alice//stash")),
			get_account_id(from_seed::<sr25519::Public>("Bob//stash")),
			get_account_id(from_seed::<sr25519::Public>("Charlie//stash")),
			get_account_id(from_seed::<sr25519::Public>("Dave//stash")),
			get_account_id(from_seed::<sr25519::Public>("Eve//stash")),
			get_account_id(from_seed::<sr25519::Public>("Ferdie//stash")),
		],
		PARA_ID.into(),
	))
	.build()
}

#[cfg(feature = "local")]
pub fn local_testnet_config() -> ChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "MOS".into());
	properties.insert("tokenDecimals".into(), 18.into());
	properties.insert("ss58Format".into(), 14998.into());

	#[allow(deprecated)]
	ChainSpec::builder(
		parachain_template_runtime::WASM_BINARY
			.expect("WASM binary was not built, please build it!"),
		Extensions {
			relay_chain: RELAY_CHAIN.into(),
			// You MUST set this to the correct network!
			para_id: PARA_ID,
		},
	)
	.with_name("Local Testnet")
	.with_id("local_testnet")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_patch(testnet_genesis(
		// initial collators.
		vec![
			(
				get_account_id(from_seed::<sr25519::Public>("Alice")),
				get_collator_keys_from_seed("Alice"),
			),
			(
				get_account_id(from_seed::<sr25519::Public>("Bob")),
				get_collator_keys_from_seed("Bob"),
			),
		],
		vec![
			get_account_id(from_seed::<sr25519::Public>("Alice")),
			get_account_id(from_seed::<sr25519::Public>("Bob")),
			get_account_id(from_seed::<sr25519::Public>("Charlie")),
			get_account_id(from_seed::<sr25519::Public>("Dave")),
			get_account_id(from_seed::<sr25519::Public>("Eve")),
			get_account_id(from_seed::<sr25519::Public>("Ferdie")),
			get_account_id(from_seed::<sr25519::Public>("Alice//stash")),
			get_account_id(from_seed::<sr25519::Public>("Bob//stash")),
			get_account_id(from_seed::<sr25519::Public>("Charlie//stash")),
			get_account_id(from_seed::<sr25519::Public>("Dave//stash")),
			get_account_id(from_seed::<sr25519::Public>("Eve//stash")),
			get_account_id(from_seed::<sr25519::Public>("Ferdie//stash")),
		],
		PARA_ID.into(),
	))
	.with_protocol_id("template-local")
	.with_properties(properties)
	.build()
}

#[cfg(not(feature = "local"))]
pub fn live_config() -> ChainSpec {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "MOS".into());
	properties.insert("tokenDecimals".into(), 18.into());
	properties.insert("ss58Format".into(), 14998.into());

	// TODO: make this nicer
	let accounts = [
		from_hex("0x1ee256f0b5b975c51b62e199ae1796b342d9337aa2b1dbc9777d44107446ae1d"),
		from_hex("0x5e4c345149989cfdba0f452e5e2e132901d85ad513269fdc06a77bf205d0cf67"),
		from_hex("0xdc6b9379f2f366ea7a60dae93db47b479dfa4def30962c40428167b225e9285c"),
		from_hex("0x6ebbc72a185b1b4ebc38ca63ce142a667b816a54dcc94ed25dcdb022cecce13a"),
	];

	let aura_ids = [
		AuraId::from(from_hex(
			"0xc09b26e7a448f367fe51012fb697ee1a7d3735b1915ba2b3e3c1371686bd797d",
		)),
		AuraId::from(from_hex(
			"0xe28984679daf4acb81cf7d5f6f18e6742beb94ae4535e80450a2db9ecfde2243",
		)),
		AuraId::from(from_hex(
			"0x5add02e6523ea5294cccf6292547efc26d78205aa857d37ddfda95fc6ae38a38",
		)),
		AuraId::from(from_hex(
			"0x0472d783769d432a0961d4a0233ec8805c234fee7ca2e2d3a2965fee5f4a9642",
		)),
	];

	#[allow(deprecated)]
	ChainSpec::builder(
		parachain_template_runtime::WASM_BINARY
			.expect("WASM binary was not built, please build it!"),
		Extensions {
			relay_chain: RELAY_CHAIN.into(),
			// You MUST set this to the correct network!
			para_id: PARA_ID,
		},
	)
	.with_name("Mosaic")
	.with_id("mosaic")
	.with_chain_type(ChainType::Live)
	.with_genesis_config_patch(testnet_genesis(
		// initial collators.
		vec![
			(get_account_id(accounts[0]), aura_ids[0].clone()),
			(get_account_id(accounts[1]), aura_ids[1].clone()),
		],
		accounts.iter().map(|pubkey| get_account_id(*pubkey)).collect(),
		PARA_ID.into(),
	))
	.with_protocol_id("mosaic-chain")
	.with_properties(properties)
	.build()
}

fn testnet_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> serde_json::Value {
	serde_json::json!({
		"balances": {
			"balances": endowed_accounts.iter().cloned().map(|k| (k, 10_000_000_000_000_000_000_u64)).collect::<Vec<_>>(),
		},
		"parachainInfo": {
			"parachainId": id,
		},
		"collatorSelection": {
			"invulnerables": invulnerables.iter().cloned().map(|(acc, _)| acc).collect::<Vec<_>>(),
			"candidacyBond": EXISTENTIAL_DEPOSIT * 16,
			//"desiredCandidates": 2,
		},
		"session": {
			"keys": invulnerables
				.iter()
				.cloned()
				.map(|(acc, aura)| {
					(
						acc.clone(),                // account id
						acc,                        // validator id
						session_keys(aura), 		// session keys
					)
				})
			.collect::<Vec<_>>(),
		},
		"polkadotXcm": {
			"safeXcmVersion": Some(SAFE_XCM_VERSION),
		},
		"council_collective_membership": {
			"members": invulnerables
		},
	})
}
