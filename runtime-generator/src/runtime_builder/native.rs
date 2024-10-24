use std::{borrow::Cow, path::PathBuf, process::Command};

use anyhow::{bail, Context};

use super::RuntimeBuilder;

pub struct Builder<'a> {
	pub path: PathBuf,
	pub target_dir: PathBuf,
	pub extra_build_opts: Option<Cow<'a, str>>,
	pub override_build_opts: Option<Cow<'a, str>>,
}

impl<'a> Builder<'a> {
	// Canonical representation of project's path
	fn manifest_path(&self) -> anyhow::Result<PathBuf> {
		self.path
			.canonicalize()
			.map(|p| p.join("Cargo.toml"))
			.context("Cannot represent path as canonical, it may not exist")
	}
}

impl<'a> RuntimeBuilder for Builder<'a> {
	fn build(&self, package: &str, build_opts: Option<&str>) -> anyhow::Result<Vec<u8>> {
		let build_opts = if let Some(opts) = self.override_build_opts.as_deref() {
			Cow::Borrowed(opts)
		} else {
			let base = build_opts.unwrap_or_default();
			let extra = self.extra_build_opts.as_deref().unwrap_or_default();
			Cow::Owned(format!("{base} {extra}"))
		};

		let command = format!(
			"cargo build --release -p {package} \
			       --manifest-path {manifest_path} \
			       --target-dir {target_dir} \
			       --color=always \
			       {build_opts}",
			manifest_path = self.manifest_path()?.display(),
			target_dir = self.target_dir.display(),
			build_opts = build_opts
		);

		log::debug!("command = {command:?}");

		let status =
			Command::new("sh").arg("-c").arg(command).stdout(std::io::stderr()).status()?;

		if !status.success() {
			bail!("Could not build runtime: cargo command execution failed");
		}

		std::fs::read(format!(
			"{target_dir}/release/wbuild/{package}/{binary}.compact.compressed.wasm",
			target_dir = self.target_dir.display(),
			package = package,
			binary = package.replace('-', "_"),
		))
		.context("Could not load built wasm binary")
	}
}
