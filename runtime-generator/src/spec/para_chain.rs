use std::marker::PhantomData;

use sdk::{
	cumulus_primitives_core, pallet_balances, pallet_collator_selection, pallet_membership,
	pallet_session, pallet_xcm, sc_chain_spec, sc_service, sp_consensus_aura, sp_core,
	staging_parachain_info as parachain_info, staging_xcm,
};

use crate::{
	runtime_builder::RuntimeBuilder,
	spec::{
		common::{mainnet_accounts, properties, public_from_seed, testnet_accounts, AccountId},
		Profile,
	},
};

use anyhow::Context;
use cumulus_primitives_core::ParaId;
use hex_literal::hex;
use mosaic_chain_runtime::{RuntimeGenesisConfig, SS58Prefix, SessionKeys, EXISTENTIAL_DEPOSIT};
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
	Profile::new("para-live", live_config)
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
	let para_id = 2000;

	let genesis_config = genesis(
		vec![
			(
				public_from_seed::<sr25519::Public>("Alice").into(),
				public_from_seed::<AuraId>("Alice"),
			),
			(public_from_seed::<sr25519::Public>("Bob").into(), public_from_seed::<AuraId>("Bob")),
		],
		testnet_accounts(),
		para_id.into(),
	)?;

	Ok(Box::new(
		ChainSpec::builder(&wasm, Extensions { relay_chain: relay_chain.into(), para_id })
			.with_properties(properties(SS58Prefix::get()))
			.with_name("Mosaic Local Para Testnet")
			.with_id("mosaic-para-local")
			.with_protocol_id("mosaic-para-local")
			.with_chain_type(ChainType::Local)
			.with_genesis_config(genesis_config)
			.build(),
	))
}

pub fn live_config(builder: &dyn RuntimeBuilder) -> anyhow::Result<Box<dyn sc_service::ChainSpec>> {
	let wasm = build_runtime(builder, None)?;
	let relay_chain = "polkadot";
	let para_id = 3377;

	let accounts = mainnet_accounts();

	let genesis_config = genesis(
		vec![
			(
				accounts[0].clone(),
				AuraId::from_slice(&hex!(
					"c09b26e7a448f367fe51012fb697ee1a7d3735b1915ba2b3e3c1371686bd797d"
				))
				.expect("Constant value is correct"),
			),
			(
				accounts[1].clone(),
				AuraId::from_slice(&hex!(
					"e28984679daf4acb81cf7d5f6f18e6742beb94ae4535e80450a2db9ecfde2243"
				))
				.expect("Constant value is correct"),
			),
		],
		accounts,
		para_id.into(),
	)?;

	Ok(Box::new(
		ChainSpec::builder(&wasm, Extensions { relay_chain: relay_chain.into(), para_id })
			.with_properties(properties(SS58Prefix::get()))
			.with_name("Mosaic")
			.with_id("mosaic-para-live")
			.with_protocol_id("mosaic-para-live")
			.with_chain_type(ChainType::Live)
			.with_genesis_config(genesis_config)
			.build(),
	))
}

fn genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	para_id: ParaId,
) -> anyhow::Result<serde_json::Value> {
	let parachain_info =
		parachain_info::GenesisConfig { parachain_id: para_id, _config: PhantomData };

	let balances = pallet_balances::GenesisConfig {
		balances: endowed_accounts
			.into_iter()
			.map(|acc| (acc, 10_000_000_000_000_000_000))
			.collect(),
	};

	let council_collective_membership = pallet_membership::GenesisConfig {
		members: invulnerables
			.iter()
			.cloned()
			.map(|(acc, _)| acc)
			.collect::<Vec<_>>()
			.try_into()
			.expect("members are fewer than MaxMembers"),
		phantom: PhantomData,
	};

	let collator_selection = pallet_collator_selection::GenesisConfig {
		invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
		candidacy_bond: EXISTENTIAL_DEPOSIT * 16,
		desired_candidates: 0,
	};

	let session = pallet_session::GenesisConfig {
		keys: invulnerables
			.into_iter()
			.map(|(acc, aura)| (acc.clone(), acc, SessionKeys { aura }))
			.collect(),
		..Default::default()
	};

	let polkadot_xcm = pallet_xcm::GenesisConfig {
		safe_xcm_version: Some(staging_xcm::prelude::XCM_VERSION),
		_config: PhantomData,
	};

	let genesis_config = RuntimeGenesisConfig {
		parachain_info,
		balances,
		council_collective_membership,
		collator_selection,
		session,
		polkadot_xcm,
		..Default::default()
	};

	serde_json::to_value(genesis_config).context("Could not represent genesis config as json value")
}
