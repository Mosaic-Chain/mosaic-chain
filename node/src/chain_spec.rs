use std::marker::PhantomData;

use mosaic_chain_runtime::{
	opaque::SessionKeys, AccountId, Balance, BalancesConfig, NftPermissionConfig, NftStakingConfig,
	RuntimeGenesisConfig, SessionConfig, Signature, SudoConfig, SystemConfig,
	ValidatorSubsetSelectionConfig, WASM_BINARY,
};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_nft_staking::PermissionType;
use sc_service::{ChainType, Properties};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig>;

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate an Aura authority key.
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId, ImOnlineId, AccountId) {
	(
		get_from_seed::<AuraId>(s),
		get_from_seed::<GrandpaId>(s),
		get_from_seed::<ImOnlineId>(s),
		get_account_id_from_seed::<sr25519::Public>(s),
	)
}

pub fn properties() -> Properties {
	let mut properties = Properties::new();

	properties.insert("tokenSymbol".into(), "MOS".into());
	properties.insert("tokenDecimals".into(), 18.into());
	properties.insert("ss58Format".into(), 42.into());
	properties.insert("color".into(), "#5f32ff".into());

	properties
}

pub fn development_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	Ok(ChainSpec::from_genesis(
		// Name
		"Mosaic Chain Development",
		// ID
		"dev",
		ChainType::Development,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![
					authority_keys_from_seed("Alice"),
					authority_keys_from_seed("Bob"),
					authority_keys_from_seed("Charlie"),
					authority_keys_from_seed("Dave"),
					authority_keys_from_seed("Eve"),
					authority_keys_from_seed("Ferdie"),
				],
				vec![
					(
						get_account_id_from_seed::<sr25519::Public>("Alice"),
						PermissionType::DPoS,
						true,
						100,
					),
					(
						get_account_id_from_seed::<sr25519::Public>("Bob"),
						PermissionType::DPoS,
						true,
						100,
					),
					(
						get_account_id_from_seed::<sr25519::Public>("Charlie"),
						PermissionType::DPoS,
						true,
						100,
					),
					(
						get_account_id_from_seed::<sr25519::Public>("Dave"),
						PermissionType::DPoS,
						true,
						100,
					),
					(
						get_account_id_from_seed::<sr25519::Public>("Eve"),
						PermissionType::DPoS,
						true,
						100,
					),
					(
						get_account_id_from_seed::<sr25519::Public>("Ferdie"),
						PermissionType::DPoS,
						false,
						10,
					),
				],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Pre-funded accounts
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Charlie"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
					get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
				],
				3,
				true,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		None,
		// Properties
		Some(properties()),
		// Extensions
		None,
	))
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

	Ok(ChainSpec::from_genesis(
		// Name
		"Mosaic Chain Local Testnet",
		// ID
		"local_testnet",
		ChainType::Local,
		move || {
			testnet_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![
					authority_keys_from_seed("Alice"),
					authority_keys_from_seed("Bob"),
					authority_keys_from_seed("Charlie"),
					authority_keys_from_seed("Dave"),
					authority_keys_from_seed("Eve"),
					authority_keys_from_seed("Ferdie"),
				],
				vec![
					(
						get_account_id_from_seed::<sr25519::Public>("Alice"),
						PermissionType::DPoS,
						true,
						100,
					),
					(
						get_account_id_from_seed::<sr25519::Public>("Bob"),
						PermissionType::DPoS,
						true,
						100,
					),
					(
						get_account_id_from_seed::<sr25519::Public>("Charlie"),
						PermissionType::DPoS,
						true,
						100,
					),
					(
						get_account_id_from_seed::<sr25519::Public>("Dave"),
						PermissionType::DPoS,
						true,
						100,
					),
					(
						get_account_id_from_seed::<sr25519::Public>("Eve"),
						PermissionType::DPoS,
						true,
						100,
					),
					(
						get_account_id_from_seed::<sr25519::Public>("Ferdie"),
						PermissionType::DPoS,
						false,
						100,
					),
				],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Pre-funded accounts
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Charlie"),
					get_account_id_from_seed::<sr25519::Public>("Dave"),
					get_account_id_from_seed::<sr25519::Public>("Eve"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
					get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
					get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
					get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
				],
				3,
				true,
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		None,
		// Properties
		Some(properties()),
		// Extensions
		None,
	))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AuraId, GrandpaId, ImOnlineId, AccountId)>,
	initial_permission_holders: Vec<(AccountId, PermissionType, bool, Balance)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	initial_subset_size: u64,
	_enable_println: bool,
) -> RuntimeGenesisConfig {
	RuntimeGenesisConfig {
		system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
			..Default::default()
		},
		nft_delegation: Default::default(),
		balances: BalancesConfig {
			// Configure endowed accounts with initial balance of 1 << 60.
			balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 60)).collect(),
		},
		aura: Default::default(),
		assets: Default::default(),
		grandpa: Default::default(),
		im_online: Default::default(),
		sudo: SudoConfig {
			// Assign network admin rights.
			key: Some(root_key),
		},
		transaction_payment: Default::default(),
		nft_permission: NftPermissionConfig {
			unstaked_permission_holders: initial_permission_holders
				.iter()
				.cloned()
				.filter_map(|(acc, perm, bound, nominal)| (!bound).then_some((acc, perm, nominal)))
				.collect(),
		},
		nft_staking: NftStakingConfig {
			initial_staking_validators: initial_permission_holders
				.into_iter()
				.filter_map(|(acc, perm, bound, nominal)| bound.then_some((acc, perm, nominal)))
				.collect(),
		},
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| {
					(
						x.3.clone(),
						x.3.clone(),
						SessionKeys {
							aura: x.0.clone(),
							grandpa: x.1.clone(),
							im_online: x.2.clone(),
						},
					)
				})
				.collect::<Vec<_>>(),
		},
		validator_subset_selection: ValidatorSubsetSelectionConfig {
			initial_subset_size,
			_phantom: PhantomData,
		},
	}
}
