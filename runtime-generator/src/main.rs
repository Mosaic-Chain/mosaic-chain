use std::process::Command;

use clap::Parser;

use runtime_generator::cli;

const GIT_STATE_HASH: &str = env!("GIT_STATE_HASH");

fn main() -> anyhow::Result<()> {
	env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

	check_repo_state();

	cli::Cli::parse().exec()
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
