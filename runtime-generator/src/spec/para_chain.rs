use std::marker::PhantomData;

use pallet_nft_staking::PermissionType;
use sdk::{
	cumulus_primitives_core, pallet_balances, pallet_session, pallet_xcm, sc_chain_spec,
	sc_service, sp_consensus_aura, sp_core, staging_parachain_info as parachain_info, staging_xcm,
};

use crate::{
	runtime_builder::RuntimeBuilder,
	spec::{
		common::{
			funds, membership_config, properties, public_from_seed, testnet_accounts, AccountId,
		},
		Profile,
	},
};

use anyhow::Context;
use cumulus_primitives_core::ParaId;
use hex_literal::hex;
use mosaic_chain_runtime::{
	opaque::SessionKeys,
	params::currency::{Balance, MOSAIC},
	RuntimeGenesisConfig,
};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, ByteArray};

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
	pub relay_chain: String,
	pub para_id: u32,
}

pub type ChainSpec = sc_service::GenericChainSpec<Extensions>;

inventory::submit! {
	Profile::new("para-local", local_config)
}

inventory::submit! {
	Profile::new("para-devnet", devnet_config)
}

inventory::submit! {
	Profile::new("para-mainnet", mainnet_config)
}

pub fn mainnet_accounts() -> Vec<AccountId> {
	vec![
		hex!("1ee256f0b5b975c51b62e199ae1796b342d9337aa2b1dbc9777d44107446ae1d").into(),
		hex!("5e4c345149989cfdba0f452e5e2e132901d85ad513269fdc06a77bf205d0cf67").into(),
		hex!("dc6b9379f2f366ea7a60dae93db47b479dfa4def30962c40428167b225e9285c").into(),
		hex!("6ebbc72a185b1b4ebc38ca63ce142a667b816a54dcc94ed25dcdb022cecce13a").into(),
	]
}

pub fn devnet_accounts() -> Vec<AccountId> {
	vec![
		hex!("40dfa76970d4764caf433a9fab6d182e1e4d930e708e1c4f2037745126f4c368").into(),
		hex!("ecf63d434c813fead2bd9f609505a4f4c2458353038b1c207d71ef4247457316").into(),
		hex!("94ceb7088b35c85435a29912a8a5e6245fe3b8b8dd448f2fefcdd3688f86f55a").into(),
		hex!("720122b86ea8eded8598ed8daf51ccd02bfd437cc5a521d9fa54f4e21e38ea39").into(),
	]
}

fn authority_keys_from_seed(s: &str) -> (AuraId, ImOnlineId, AccountId) {
	(
		public_from_seed::<AuraId>(s),
		public_from_seed::<ImOnlineId>(s),
		public_from_seed::<sr25519::Public>(s).into(),
	)
}

fn authority_keys_from_slice(
	data: &[u8; 32],
	account: AccountId,
) -> (AuraId, ImOnlineId, AccountId) {
	(
		AuraId::from_slice(data).expect("Constant value is correct"),
		ImOnlineId::from_slice(data).expect("Constant value is correct"),
		account,
	)
}

fn build_runtime(
	builder: &dyn RuntimeBuilder,
	extra_opts: Option<&str>,
) -> anyhow::Result<Vec<u8>> {
	let opts = format!("-F build-wasm {}", extra_opts.unwrap_or_default());
	builder.build("mosaic-chain-runtime", Some(&opts))
}

pub fn local_config(
	builder: &dyn RuntimeBuilder,
) -> anyhow::Result<Box<dyn sc_service::ChainSpec>> {
	let wasm = build_runtime(builder, Some("-F local"))?;
	let relay_chain = "paseo-local";
	let para_id = 3377;

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
			.map(|acc| (acc, PermissionType::DPoS, 100 * MOSAIC))
			.collect(),
		{
			let mut members = testnet_accounts();
			members.truncate(3);
			members
		},
		testnet_accounts(),
		public_from_seed::<sr25519::Public>("MintingAuthority"),
		para_id.into(),
	)?;

	Ok(Box::new(
		ChainSpec::builder(&wasm, Extensions { relay_chain: relay_chain.into(), para_id })
			.with_properties(properties(0, "TMOS"))
			.with_name("Mosaic Chain Local Testnet")
			.with_id("mosaic-local")
			.with_protocol_id("mosaic-local")
			.with_chain_type(ChainType::Local)
			.with_genesis_config(genesis_config)
			.build(),
	))
}

pub fn devnet_config(
	builder: &dyn RuntimeBuilder,
) -> anyhow::Result<Box<dyn sc_service::ChainSpec>> {
	let wasm = build_runtime(builder, None)?;
	let relay_chain = "paseo";
	let para_id = 3377;

	let accounts = devnet_accounts();
	let genesis_config = genesis(
		vec![
			authority_keys_from_slice(
				&hex!("fe3aa900e2170308b30070936b9bef3923f52f94a8073ffff13cd7ec9f9da01c"),
				accounts[0].clone(),
			),
			authority_keys_from_slice(
				&hex!("e84ea0bb130c6c476130145d695efd99a83c7f126f31b3b5f353df793ddf5008"),
				accounts[1].clone(),
			),
			authority_keys_from_slice(
				&hex!("d24dc34aa193635f614efee940b5006052598918b3332a81fef585e0ee0ee95d"),
				accounts[2].clone(),
			),
			authority_keys_from_slice(
				&hex!("74e4f2004802188e6b07e8a7443a393e6b920d12dbc4967b88b8dcc65bc6b75f"),
				accounts[3].clone(),
			),
		],
		accounts.iter().map(|acc| (acc.clone(), PermissionType::PoS, 0)).collect(),
		// TODO: use separate devnet council member addresses
		vec![
			hex!("40dfa76970d4764caf433a9fab6d182e1e4d930e708e1c4f2037745126f4c368").into(),
			hex!("ecf63d434c813fead2bd9f609505a4f4c2458353038b1c207d71ef4247457316").into(),
			hex!("94ceb7088b35c85435a29912a8a5e6245fe3b8b8dd448f2fefcdd3688f86f55a").into(),
			hex!("720122b86ea8eded8598ed8daf51ccd02bfd437cc5a521d9fa54f4e21e38ea39").into(),
		],
		accounts.clone(),
		hex!("724057d84b455a2fef18d9d1ddf6a0b524d69954a0bf997a902a64dce6d22d35").into(),
		para_id.into(),
	)?;

	Ok(Box::new(
		ChainSpec::builder(&wasm, Extensions { relay_chain: relay_chain.into(), para_id })
			.with_properties(properties(0, "TMOS"))
			.with_name("Mosaic Chain Devnet")
			.with_id("mosaic-devnet")
			.with_protocol_id("mosaic-devnet")
			.with_chain_type(ChainType::Live)
			.with_genesis_config(genesis_config)
			.build(),
	))
}

pub fn mainnet_config(
	builder: &dyn RuntimeBuilder,
) -> anyhow::Result<Box<dyn sc_service::ChainSpec>> {
	let wasm = build_runtime(builder, Some("-F mainnet"))?;
	let relay_chain = "polkadot";
	let para_id = 3377;

	let accounts = mainnet_accounts();

	let genesis_config = genesis(
		vec![
			authority_keys_from_slice(
				&hex!("c09b26e7a448f367fe51012fb697ee1a7d3735b1915ba2b3e3c1371686bd797d"),
				accounts[0].clone(),
			),
			authority_keys_from_slice(
				&hex!("e28984679daf4acb81cf7d5f6f18e6742beb94ae4535e80450a2db9ecfde2243"),
				accounts[1].clone(),
			),
			authority_keys_from_slice(
				&hex!("5add02e6523ea5294cccf6292547efc26d78205aa857d37ddfda95fc6ae38a38"),
				accounts[2].clone(),
			),
			authority_keys_from_slice(
				&hex!("0472d783769d432a0961d4a0233ec8805c234fee7ca2e2d3a2965fee5f4a9642"),
				accounts[3].clone(),
			),
		],
		accounts.iter().map(|acc| (acc.clone(), PermissionType::PoS, 0)).collect(),
		vec![
			hex!("40dfa76970d4764caf433a9fab6d182e1e4d930e708e1c4f2037745126f4c368").into(),
			hex!("ecf63d434c813fead2bd9f609505a4f4c2458353038b1c207d71ef4247457316").into(),
			hex!("94ceb7088b35c85435a29912a8a5e6245fe3b8b8dd448f2fefcdd3688f86f55a").into(),
			hex!("720122b86ea8eded8598ed8daf51ccd02bfd437cc5a521d9fa54f4e21e38ea39").into(),
		],
		accounts.clone(),
		hex!("ca494ec6c3f000a1ad37c0fd8ef176007a54480c5328cdd1003f4ec1324d1927").into(),
		para_id.into(),
	)?;

	Ok(Box::new(
		ChainSpec::builder(&wasm, Extensions { relay_chain: relay_chain.into(), para_id })
			.with_properties(properties(0, "MOS"))
			.with_name("Mosaic Chain")
			.with_id("mosaic-mainnet")
			.with_protocol_id("mosaic-mainnet")
			.with_chain_type(ChainType::Live)
			.with_genesis_config(genesis_config)
			.build(),
	))
}

fn genesis(
	initial_authorities: Vec<(AuraId, ImOnlineId, AccountId)>,
	initial_staking_validators: Vec<(AccountId, PermissionType, Balance)>,
	council_members: Vec<AccountId>,
	endowed_accounts: Vec<AccountId>,
	minting_authority: sr25519::Public,
	para_id: ParaId,
) -> anyhow::Result<serde_json::Value> {
	let parachain_info =
		parachain_info::GenesisConfig { parachain_id: para_id, _config: PhantomData };

	let polkadot_xcm = pallet_xcm::GenesisConfig {
		safe_xcm_version: Some(staging_xcm::prelude::XCM_VERSION),
		_config: PhantomData,
	};

	let endowed = endowed_accounts.into_iter().map(|k| (k, 10 * MOSAIC));

	let balances = pallet_balances::GenesisConfig { balances: endowed.chain(funds()).collect() };

	let staking_incentive =
		pallet_staking_incentive::GenesisConfig { incentive_pool: 500_000_000 * MOSAIC };

	let session = pallet_session::GenesisConfig {
		keys: initial_authorities
			.into_iter()
			.map(|x| {
				(
					x.2.clone(),
					x.2.clone(),
					SessionKeys { aura: x.0.clone(), im_online: x.1.clone() },
				)
			})
			.collect(),
		..Default::default()
	};

	let nft_staking = pallet_nft_staking::GenesisConfig { initial_staking_validators };

	let airdrop = pallet_airdrop::GenesisConfig { minting_authority, _phantom: PhantomData };

	let genesis_config = RuntimeGenesisConfig {
		balances,
		nft_staking,
		session,
		airdrop,
		council_membership: membership_config(&council_members),
		development_membership: membership_config(&council_members),
		financial_membership: membership_config(&council_members),
		community_membership: membership_config(&council_members),
		team_and_advisors_membership: membership_config(&council_members),
		security_membership: membership_config(&council_members),
		education_membership: membership_config(&council_members),
		staking_incentive,
		parachain_info,
		polkadot_xcm,
		..Default::default()
	};

	serde_json::to_value(genesis_config).context("Could not represent genesis config as json value")
}
