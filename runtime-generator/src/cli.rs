use clap::{Parser, Subcommand};
use std::{borrow::Cow, path::PathBuf};

#[derive(Parser)]
pub struct Cli {
	/// Choose an alternative image. Beware to choose an image that is
	/// compatible with the original srtool image. Using a random image,
	/// you take the risk to NOT produce exactly the same deterministic
	/// result as srtool.
	#[arg(short, long, default_value = "docker.io/paritytech/srtool", global = true)]
	pub image: Cow<'static, str>,

	/// Choose a fixed image version. Beware to choose a tag
	/// compatible with the spesific version of runtime built.
	#[arg(short, long, default_value = "1.81.0-0.16.0", global = true)]
	pub tag: Cow<'static, str>,

	#[clap(subcommand)]
	pub subcmd: SubCommand,
}

#[derive(Subcommand)]
pub enum SubCommand {
	/// Simply pull the srtool image and do not run anything else
	Pull(PullOpts),

	/// Provide information about the srtool container used
	Info(InfoOpts),

	/// Start a new srtool container to build your runtime
	Build(BuildOpts),
}

#[derive(Parser)]
pub struct PullOpts;

#[derive(Parser)]
pub struct InfoOpts;

#[derive(Parser)]
pub struct BuildOpts {
	/// Chain spec to generate.
	pub chain: Cow<'static, str>,

	/// Generate the chain spec as raw
	#[arg(long)]
	pub raw: bool,

	#[clap(flatten)]
	pub srtool_opts: SrtoolOpts,
}

#[derive(Parser)]
pub struct SrtoolOpts {
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

	/// Passing this flag allows completely disabling caching.
	/// As a result, no cargo-home will be mounted to the srtool image.
	/// There is no known issue with having the cache ON, this is why it is the default.
	#[arg(long)]
	pub no_cache: bool,

	/// Do not run container image as root.
	#[arg(long)]
	pub no_root: bool,
}
