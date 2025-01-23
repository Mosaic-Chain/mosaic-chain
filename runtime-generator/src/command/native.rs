use std::process::Command;

use anyhow::anyhow;

use crate::{
	cli::native::{BuildOpts, Cli, Subcommand},
	runtime_builder::NativeBuilder,
};

pub fn exec(cli: Cli) -> anyhow::Result<()> {
	match cli.subcmd {
		Subcommand::Info => info(),
		Subcommand::Build(opts) => build(opts),
	}
}

fn info() -> anyhow::Result<()> {
	println!("=== Rustc version: ===");
	run_cmd("rustc --version --verbose")?;

	println!("\n=== Cargo version: ===");
	run_cmd("cargo --version --verbose")?;
	Ok(())
}

fn build(opts: BuildOpts) -> anyhow::Result<()> {
	let builder = NativeBuilder {
		path: opts.common.path,
		target_dir: opts.target_dir,
		out_wasm: opts.common.out_wasm,
		extra_build_opts: opts.common.extra_build_opts,
		override_build_opts: opts.common.override_build_opts,
	};

	super::build(&builder, &opts.common.profile, opts.common.raw)
}

fn run_cmd(cmd: &str) -> anyhow::Result<()> {
	let status = Command::new("sh").arg("-c").arg(cmd).status()?;

	if status.success() {
		Ok(())
	} else {
		Err(anyhow!("Could not run command: {cmd}"))
	}
}
