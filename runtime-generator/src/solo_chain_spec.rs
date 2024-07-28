use crate::{
	common::{mainnet_accounts, properties, public_from_seed, testnet_accounts, AccountId},
	runtime_builder::RuntimeBuilder,
};

use anyhow::Context;

use mosaic_testnet_solo_runtime::{opaque::SessionKeys, Balance, RuntimeGenesisConfig, SS58Prefix};

use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_nft_staking::PermissionType;
use sc_service::ChainType;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::sr25519;
use std::marker::PhantomData;

pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig>;

fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId, ImOnlineId, AccountId) {
	(
		public_from_seed::<AuraId>(s),
		public_from_seed::<GrandpaId>(s),
		public_from_seed::<ImOnlineId>(s),
		public_from_seed::<sr25519::Public>(s).into(),
	)
}

fn build_runtime(builder: &dyn RuntimeBuilder) -> anyhow::Result<Vec<u8>> {
	builder.build("mosaic-testnet-solo-runtime", Some("-F include-wasm"))
}

pub fn local_config(
	builder: &dyn RuntimeBuilder,
) -> anyhow::Result<Box<dyn sc_service::ChainSpec>> {
	let genesis_config = genesis(
		vec![
			authority_keys_from_seed("Alice"),
			authority_keys_from_seed("Bob"),
			authority_keys_from_seed("Charlie"),
			authority_keys_from_seed("Dave"),
			authority_keys_from_seed("Eve"),
			authority_keys_from_seed("Ferdie"),
		],
		testnet_accounts()
			.into_iter()
			.take(6)
			.enumerate()
			.map(|(i, acc)| (acc, PermissionType::DPoS, i != 5, 100))
			.collect(),
		{
			let mut members = testnet_accounts();
			members.truncate(3);
			members
		},
		testnet_accounts(),
		3,
	)?;

	Ok(Box::new(
		ChainSpec::builder(&build_runtime(builder)?, None)
			.with_properties(properties(SS58Prefix::get().into()))
			.with_name("Mosaic Local Solo Testnet")
			.with_id("mosaic-solo-local")
			.with_protocol_id("mosaic-solo-local")
			.with_chain_type(ChainType::Local)
			.with_genesis_config(genesis_config)
			.build(),
	))
}

pub fn live_config(builder: &dyn RuntimeBuilder) -> anyhow::Result<Box<dyn sc_service::ChainSpec>> {
	let genesis_config = genesis(
		vec![
			authority_keys_from_seed("Alice"),
			authority_keys_from_seed("Bob"),
			authority_keys_from_seed("Charlie"),
			authority_keys_from_seed("Dave"),
			authority_keys_from_seed("Eve"),
			authority_keys_from_seed("Ferdie"),
		],
		mainnet_accounts()
			.into_iter()
			// TODO: revise our accounts' permission and nominal value
			.map(|acc| (acc, PermissionType::DPoS, true, 100))
			.collect(),
		mainnet_accounts(), // TODO: this will need to be changed to our accounts
		mainnet_accounts(),
		250,
	)?;

	Ok(Box::new(
		ChainSpec::builder(&build_runtime(builder)?, None)
			.with_properties(properties(SS58Prefix::get().into()))
			.with_name("Mosaic Solo Testnet")
			.with_id("mosaic-solo-live")
			.with_protocol_id("mosaic-solo-live")
			.with_chain_type(ChainType::Live)
			.with_genesis_config(genesis_config)
			.build(),
	))
}
fn genesis(
	initial_authorities: Vec<(AuraId, GrandpaId, ImOnlineId, AccountId)>,
	initial_permission_holders: Vec<(AccountId, PermissionType, bool, Balance)>,
	council_members: Vec<AccountId>,
	endowed_accounts: Vec<AccountId>,
	initial_subset_size: u64,
) -> anyhow::Result<serde_json::Value> {
	let balances = pallet_balances::GenesisConfig {
		balances: endowed_accounts.into_iter().map(|k| (k, 10_000_000_000_000_000_000)).collect(),
	};

	let session = pallet_session::GenesisConfig {
		keys: initial_authorities
			.into_iter()
			.map(|x| {
				(
					x.3.clone(),
					x.3.clone(),
					SessionKeys { aura: x.0.clone(), grandpa: x.1.clone(), im_online: x.2.clone() },
				)
			})
			.collect(),
	};

	let council_collective_membership = pallet_membership::GenesisConfig {
		members: council_members.try_into().expect("members are fewer than MaxMembers"),
		phantom: PhantomData,
	};

	let nft_permission = pallet_nft_permission::GenesisConfig {
		unstaked_permission_holders: initial_permission_holders
			.iter()
			.cloned()
			.filter_map(|(acc, perm, bound, nominal)| (!bound).then_some((acc, perm, nominal)))
			.collect(),
	};

	let nft_staking = pallet_nft_staking::GenesisConfig {
		initial_staking_validators: initial_permission_holders
			.into_iter()
			.filter_map(|(acc, perm, bound, nominal)| bound.then_some((acc, perm, nominal)))
			.collect(),
	};

	let validator_subset_selection = pallet_validator_subset_selection::GenesisConfig {
		initial_subset_size,
		_phantom: PhantomData,
	};

	let genesis_config = RuntimeGenesisConfig {
		balances,
		nft_permission,
		nft_staking,
		validator_subset_selection,
		session,
		council_collective_membership,
		..Default::default()
	};

	serde_json::to_value(genesis_config).context("Could not represent genesis config as json value")
}
