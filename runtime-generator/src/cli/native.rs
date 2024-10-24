use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
pub struct Cli {
	#[clap(subcommand)]
	pub subcmd: Subcommand,
}

#[derive(clap::Subcommand)]
pub enum Subcommand {
	/// Provide information about the local build environment
	Info,

	/// Start building your runtime with the locally installed toolchain.
	/// WARNING: the build will not be deterministic
	Build(BuildOpts),
}

#[derive(Parser)]
pub struct BuildOpts {
	#[clap(flatten)]
	pub common: super::CommonBuildOpts,

	/// Target directory used during the build
	#[arg(long, default_value = "target")]
	pub target_dir: PathBuf,
}
