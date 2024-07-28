use hex_literal::hex;
use sc_service::Properties;
use sp_core::{sr25519, Pair, Public};

pub type AccountId = sp_core::crypto::AccountId32;

/// Chain properties
pub fn properties(ss58prefix: u16) -> Properties {
	let mut properties = Properties::new();

	properties.insert("tokenSymbol".into(), "MOS".into());
	properties.insert("tokenDecimals".into(), 18.into());
	properties.insert("ss58Format".into(), ss58prefix.into());
	properties.insert("color".into(), "#5f32ff".into());

	properties
}

/// Helper function to generate a crypto pair from seed
pub fn public_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{seed}"), None)
		.expect("static values are valid; qed")
		.public()
}

pub fn testnet_accounts() -> Vec<AccountId> {
	let accounts = [
		"Alice",
		"Bob",
		"Charlie",
		"Dave",
		"Eve",
		"Ferdie",
		"Alice//stash",
		"Bob//stash",
		"Charlie//stash",
		"Dave//stash",
		"Eve//stash",
		"Ferdie//stash",
	];

	accounts
		.into_iter()
		.map(public_from_seed::<sr25519::Public>)
		.map(Into::into)
		.collect()
}

pub fn mainnet_accounts() -> Vec<AccountId> {
	vec![
		hex!("1ee256f0b5b975c51b62e199ae1796b342d9337aa2b1dbc9777d44107446ae1d").into(),
		hex!("5e4c345149989cfdba0f452e5e2e132901d85ad513269fdc06a77bf205d0cf67").into(),
		hex!("dc6b9379f2f366ea7a60dae93db47b479dfa4def30962c40428167b225e9285c").into(),
		hex!("6ebbc72a185b1b4ebc38ca63ce142a667b816a54dcc94ed25dcdb022cecce13a").into(),
	]
}
