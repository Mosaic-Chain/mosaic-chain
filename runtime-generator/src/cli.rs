use std::{borrow::Cow, path::PathBuf};

use clap::{Parser, Subcommand};

pub mod native;
pub mod srtool;

#[derive(Parser)]
pub struct Cli {
	#[clap(subcommand)]
	pub toolchain: Toolchain,
}

#[derive(Subcommand)]
pub enum Toolchain {
	/// Use the locally installed toolchain to generate the runtime.
	/// The build will be non-deterministic and thus should not be used in production.
	Native(native::Cli),

	/// Use srtool to generate the runtime.
	/// This will result in a deterministic build and thus can be used in production.
	Srtool(srtool::Cli),
}

#[derive(Parser)]
pub struct CommonBuildOpts {
	/// Chain spec to generate.

	// Setting a default causes the display of supported profiles
	// Its a bit of a hack, but improves usability
	#[arg(default_value = "")]
	pub profile: Cow<'static, str>,

	/// Generate the chain spec as raw
	#[arg(long)]
	pub raw: bool,

	/// By default, this tool will work in the current folder.
	/// If the project is located in another location, you can pass it here.
	#[arg(long, default_value = ".")]
	pub path: PathBuf,

	/// Pass options to cargo directly in addition to default ones.
	#[arg(long)]
	pub extra_build_opts: Option<Cow<'static, str>>,

	/// Pass options to cargo directly overriding the defaults.
	#[arg(long, conflicts_with = "extra_build_opts")]
	pub override_build_opts: Option<Cow<'static, str>>,
}
