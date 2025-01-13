use sdk::{pallet_membership, sc_service, sp_core};

use std::marker::PhantomData;

use sc_service::Properties;
use sp_core::{sr25519, Pair, Public};

use mosaic_chain_runtime::{funds, Balance, MOSAIC};

pub type AccountId = sp_core::crypto::AccountId32;

/// Chain properties
#[must_use]
pub fn properties(ss58prefix: u16, token_symbol: &str) -> Properties {
	let mut properties = Properties::new();

	properties.insert("tokenSymbol".into(), token_symbol.into());
	properties.insert("tokenDecimals".into(), 18.into());
	properties.insert("ss58Format".into(), ss58prefix.into());
	properties.insert("color".into(), "#5f32ff".into());

	properties
}

/// Helper function to generate a crypto pair from seed
#[must_use]
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

pub fn funds() -> impl Iterator<Item = (AccountId, Balance)> {
	[
		(funds::treasury::Account::get(), 10_000_000 * MOSAIC),
		(funds::development_fund::Account::get(), 24_000_000 * MOSAIC),
		(funds::financial_fund::Account::get(), 20_000_000 * MOSAIC),
		(funds::community_fund::Account::get(), 20_000_000 * MOSAIC),
		(funds::team_and_advisors_fund::Account::get(), 8_000_000 * MOSAIC),
		(funds::security_fund::Account::get(), 4_000_000 * MOSAIC),
		(funds::education_fund::Account::get(), 2_400_000 * MOSAIC),
	]
	.into_iter()
}

pub fn membership_config<T, I>(members: &[T::AccountId]) -> pallet_membership::GenesisConfig<T, I>
where
	T: pallet_membership::Config<I>,
{
	pallet_membership::GenesisConfig {
		members: members.to_vec().try_into().expect("members are fewer than MaxMembers"),
		phantom: PhantomData,
	}
}
