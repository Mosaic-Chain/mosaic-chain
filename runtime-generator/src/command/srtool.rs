use std::{borrow::Cow, process::Command};

use anyhow::{anyhow, Context};

use crate::{
	cli::srtool::{BuildOpts, Cli, Subcommand},
	runtime_builder::SrtoolBuilder,
};

pub fn exec(Cli { image, tag, subcmd }: Cli) -> anyhow::Result<()> {
	match subcmd {
		Subcommand::Pull => pull(&image, &tag),
		Subcommand::Info => info(&image, &tag),
		Subcommand::Build(opts) => build(image, tag, opts),
	}
}

fn pull(image: &str, tag: &str) -> anyhow::Result<()> {
	log::debug!("Pulling docker image: {}:{}", image, tag);

	let status = Command::new("sh")
		.arg("-c")
		.arg(format!("docker pull {image}:{tag}"))
		.status()?;

	if status.success() {
		Ok(())
	} else {
		Err(anyhow!("Could not pull srtool image"))
	}
}

fn info(image: &str, tag: &str) -> anyhow::Result<()> {
	let current_tag = srtool_lib::fetch_image_tag()?;
	println!("Latest image: {image}:{current_tag}");
	println!("Current image: {image}:{tag}");

	log::debug!("Fetching tooling info from inside of current image");
	let status = Command::new("sh")
		.arg("-c")
		.arg(format!("docker run --name srtool --rm {image}:{tag} version"))
		.status()?;

	if status.success() {
		Ok(())
	} else {
		Err(anyhow!("Could not check version"))
	}
}

fn build<'a>(image: Cow<'a, str>, tag: Cow<'a, str>, opts: BuildOpts) -> anyhow::Result<()> {
	install_ctrlc()?;

	log::info!("Building using srtool {image}:{tag}");
	let builder = SrtoolBuilder {
		image,
		tag,
		path: opts.common.path,
		extra_build_opts: opts.common.extra_build_opts,
		override_build_opts: opts.common.override_build_opts,
		no_root: opts.no_root,
		no_cache: opts.no_cache,
	};

	super::build(&builder, &opts.common.profile, opts.common.raw)
}

fn install_ctrlc() -> anyhow::Result<()> {
	ctrlc::set_handler(move || {
		log::info!("Killing srtool container, your build may not be finished...");

		let _ = Command::new("sh")
			.arg("-c")
			.arg("docker rm -f srtool")
			.spawn()
			.expect("failed to execute cleaning process")
			.wait();

		log::info!("Exiting");
		std::process::exit(0);
	})
	.context("Could not set up ctrlc handler")
}
