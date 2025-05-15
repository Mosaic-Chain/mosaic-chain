use sdk::{frame_benchmarking_cli, sc_cli};
use std::borrow::Cow;

#[derive(Debug, clap::Parser)]
pub struct Cli {
	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,

	/// The Sentry DSN used by the node
	#[arg(long, env, requires = "sentry_environment")]
	pub sentry_dsn: Option<Cow<'static, str>>,

	/// The environment the node is deployed to
	#[arg(long, env, requires = "sentry_dsn")]
	pub sentry_environment: Option<Cow<'static, str>>,

	#[clap(flatten)]
	pub run: sc_cli::RunCmd,
}

#[derive(Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Subcommand {
	/// Key management cli utilities
	#[command(subcommand)]
	Key(sc_cli::KeySubcommand),

	/// Build a chain specification.
	BuildSpec(sc_cli::BuildSpecCmd),

	/// Validate blocks.
	CheckBlock(sc_cli::CheckBlockCmd),

	/// Export blocks.
	ExportBlocks(sc_cli::ExportBlocksCmd),

	/// Export the state of a given block into a chain spec.
	ExportState(sc_cli::ExportStateCmd),

	/// Import blocks.
	ImportBlocks(sc_cli::ImportBlocksCmd),

	/// Remove the whole chain.
	PurgeChain(sc_cli::PurgeChainCmd),

	/// Revert the chain to a previous state.
	Revert(sc_cli::RevertCmd),

	/// Sub-commands concerned with benchmarking.
	#[command(subcommand)]
	Benchmark(Box<frame_benchmarking_cli::BenchmarkCmd>),

	/// Try-runtime has migrated to a standalone CLI
	/// (<https://github.com/paritytech/try-runtime-cli>). The subcommand exists as a stub and
	/// deprecation notice. It will be removed entirely some time after Janurary 2024.
	TryRuntime,

	/// Db meta columns information.
	ChainInfo(sc_cli::ChainInfoCmd),
}
