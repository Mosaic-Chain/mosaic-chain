use std::borrow::Cow;

use clap::Parser;

#[derive(Parser)]
pub struct Cli {
	/// Choose an alternative image. Beware to choose an image that is
	/// compatible with the original srtool image. Using a random image,
	/// you take the risk to NOT produce exactly the same deterministic
	/// result as srtool.
	#[arg(short, long, default_value = "docker.io/paritytech/srtool", global = true)]
	pub image: Cow<'static, str>,

	/// Choose a fixed image version. Beware to choose a tag
	/// compatible with the specific version of runtime built.
	#[arg(short, long, default_value = "1.88.0-0.18.3", global = true)]
	pub tag: Cow<'static, str>,

	#[clap(subcommand)]
	pub subcmd: Subcommand,
}

#[derive(clap::Subcommand)]
pub enum Subcommand {
	/// Simply pull the srtool image and do not run anything else
	Pull,

	/// Provide information about the srtool container used
	Info,

	/// Start a new srtool container to build your runtime
	Build(BuildOpts),
}

#[derive(Parser)]
pub struct BuildOpts {
	#[clap(flatten)]
	pub common: super::CommonBuildOpts,

	/// Passing this flag allows completely disabling caching.
	/// As a result, no cargo-home will be mounted to the srtool image.
	/// There is no known issue with having the cache ON, this is why it is the default.
	#[arg(long)]
	pub no_cache: bool,

	/// Do not run container image as root.
	#[arg(long)]
	pub no_root: bool,
}
