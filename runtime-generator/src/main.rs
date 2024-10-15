use std::{collections::HashMap, process::Command};

use sdk::sc_chain_spec::ChainSpec;

use anyhow::{anyhow, Context};
use clap::Parser;
use runtime_builder::{RuntimeBuilder, SrtoolBuilder};

mod cli;
mod common;
mod para_chain_spec;
mod runtime_builder;
mod solo_chain_spec;

const GIT_STATE_HASH: &str = env!("GIT_STATE_HASH");

fn main() -> anyhow::Result<()> {
	env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

	check_repo_state();

	ctrlc::set_handler(move || {
		handle_exit();
	})
	.context("Could not set up ctrlc handler")?;

	let cli = cli::Cli::parse();

	match cli.subcmd {
		cli::SubCommand::Pull(_) => {
			log::debug!("Pulling docker image: {}:{}", cli.image, cli.tag);

			let status = Command::new("sh")
				.arg("-c")
				.arg(format!("docker pull {}:{}", cli.image, cli.tag))
				.status()?;

			if status.success() {
				Ok(())
			} else {
				Err(anyhow!("Could not pull srtool image"))
			}
		},
		cli::SubCommand::Info(_) => {
			let current_tag = srtool_lib::fetch_image_tag()?;
			println!("Latest image: {}:{}", cli.image, current_tag);
			println!("Current image: {}:{}", cli.image, cli.tag);

			log::debug!("Fetching tooling info from inside of current image");
			let status = Command::new("sh")
				.arg("-c")
				.arg(format!("docker run --name srtool --rm {}:{} version", cli.image, cli.tag))
				.status()?;

			if status.success() {
				Ok(())
			} else {
				Err(anyhow!("Could not check version"))
			}
		},
		cli::SubCommand::Build(opts) => {
			let supported_chains = HashMap::<
				_,
				Box<dyn Fn(&dyn RuntimeBuilder) -> anyhow::Result<Box<dyn ChainSpec>>>,
			>::from([
				("mosaic-solo-local", Box::new(solo_chain_spec::local_config) as Box<_>),
				("mosaic-solo-live", Box::new(solo_chain_spec::live_config) as Box<_>),
				("mosaic-para-local", Box::new(para_chain_spec::local_config) as Box<_>),
				("mosaic-para-live", Box::new(para_chain_spec::live_config) as Box<_>),
			]);

			let builder = SrtoolBuilder {
				image: cli.image,
				tag: cli.tag,
				path: opts.srtool_opts.path,
				extra_build_opts: opts.srtool_opts.extra_build_opts,
				override_build_opts: opts.srtool_opts.override_build_opts,
				no_root: opts.srtool_opts.no_root,
				no_cache: opts.srtool_opts.no_cache,
			};

			if let Some(fun) = supported_chains.get(&*opts.chain) {
				log::info!("Found supported chain profile: {}", opts.chain);

				let chain_spec = (*fun)(&builder)?.as_json(opts.raw).map_err(|e| anyhow!(e))?;
				print!("{chain_spec}");
				Ok(())
			} else {
				let supported =
					supported_chains.keys().enumerate().fold(String::new(), |c, (n, k)| {
						let extra = if n + 1 < supported_chains.len() { ", " } else { "" };
						format!("{c}{k}{extra}")
					});

				if opts.chain.ends_with(".json") {
					let chain_spec = from_json_file(&opts.chain, &supported)?
						.as_json(opts.raw)
						.map_err(|e| anyhow!(e))?;
					print!("{chain_spec}");
					Ok(())
				} else {
					Err(anyhow!("Unknown chain, only supported: {supported} or a json file"))
				}
			}
		},
	}
}

// Check current hash of HEAD and if the repo is dirty.
fn check_repo_state() {
	log::debug!("Checking repo state");

	let output = Command::new("git")
		.args(["describe", "--always", "--dirty", "--abbrev=32"])
		.output()
		.expect("Failed to execute git describe command");

	let git_hash = String::from_utf8_lossy(&output.stdout);
	let git_hash = git_hash.trim();

	if git_hash != GIT_STATE_HASH {
		log::warn!(
			"Current repository state does not match the state at compilation. ({git_hash} != {GIT_STATE_HASH})"
		);
	}
}

fn handle_exit() {
	log::info!("Killing srtool container, your build may not be finished...");

	let _ = Command::new("sh")
		.arg("-c")
		.arg("docker rm -f srtool")
		.spawn()
		.expect("failed to execute cleaning process")
		.wait();

	log::info!("Exiting");
	std::process::exit(0);
}

fn from_json_file(filepath: &str, supported: &str) -> anyhow::Result<Box<dyn ChainSpec>> {
	#[derive(Debug, serde::Deserialize)]
	struct EmptyChainSpecWithId {
		id: String,
	}

	log::info!("Constructing chainspec from an existing one");

	let path = std::path::PathBuf::from(&filepath);
	let file = std::fs::File::open(filepath)?;
	let reader = std::io::BufReader::new(file);

	let chain_spec: EmptyChainSpecWithId = serde_json::from_reader(reader)?;

	match &chain_spec.id {
		x if x.starts_with("mosaic-solo") => {
			Ok(Box::new(solo_chain_spec::ChainSpec::from_json_file(path).map_err(|e| anyhow!(e))?))
		},
		x if x.starts_with("mosaic-para") => {
			Ok(Box::new(para_chain_spec::ChainSpec::from_json_file(path).map_err(|e| anyhow!(e))?))
		},
		_ => Err(anyhow!("Unknown chain 'id' in json file. Only supported: {supported}")),
	}
}
