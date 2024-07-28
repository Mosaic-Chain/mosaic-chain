use anyhow::{anyhow, bail, Context};
use std::{borrow::Cow, path::PathBuf, process::Command};

pub trait RuntimeBuilder {
	fn build(&self, package: &str, build_opts: Option<&str>) -> anyhow::Result<Vec<u8>>;
}

pub struct SrtoolBuilder<'a> {
	pub image: Cow<'a, str>,
	pub tag: Cow<'a, str>,
	pub path: PathBuf,
	pub extra_build_opts: Option<Cow<'a, str>>,
	pub override_build_opts: Option<Cow<'a, str>>,
	pub no_root: bool,
	pub no_cache: bool,
}

impl<'a> SrtoolBuilder<'a> {
	// Canonical representation of project's path
	fn base_path(&self) -> anyhow::Result<PathBuf> {
		self.path
			.canonicalize()
			.context("Cannot represent path as canonical, it may not exist")
	}

	// Path of package in workspace relative to the project's path
	fn path_of_package(&self, package: &str) -> anyhow::Result<PathBuf> {
		let base_path = self.base_path()?;

		let metadata = cargo_metadata::MetadataCommand::new()
			.manifest_path(base_path.join("Cargo.toml"))
			.exec()
			.context("Could not fetch workspace metadata")?;

		let Some(package) = metadata.packages.iter().find(|p| p.name == package) else {
			bail!("Could not locate package: {package}");
		};

		let dir = package
			.manifest_path
			.parent()
			.ok_or(anyhow!("Package manifest with no parent directory"))?
			.strip_prefix(base_path)?;

		Ok(dir.into())
	}

	// Location of cache if desired
	fn cache_dir(&self) -> Option<PathBuf> {
		(!self.no_cache).then(|| std::env::temp_dir().join("cargo"))
	}
}

impl<'a> RuntimeBuilder for SrtoolBuilder<'a> {
	fn build(&self, package: &str, build_opts: Option<&str>) -> anyhow::Result<Vec<u8>> {
		let runtime_dir =
			self.path_of_package(package).context("Could not determine runtime path")?;

		let build_opts = if let Some(opts) = self.override_build_opts.as_deref() {
			Cow::Borrowed(opts)
		} else {
			let base = build_opts.unwrap_or_default();
			let extra = self.extra_build_opts.as_deref().unwrap_or_default();
			Cow::Owned(format!("{base} {extra}"))
		};

		let cache_mount = self
			.cache_dir()
			.map(|cache_dir| format!("-v {}:/cargo-home", cache_dir.display()))
			.unwrap_or_default();

		let image_digest = srtool_lib::get_image_digest(&self.image, &self.tag).unwrap_or_default();

		let command = format!(
			"docker run --name srtool --rm \
				-e PACKAGE='{package}' \
				-e RUNTIME_DIR='{runtime_dir}' \
				-e BUILD_OPTS='{build_opts}' \
				-e IMAGE='{image_digest}' \
				-e PROFILE=release \
				-v '{path}':/build \
				{as_root} \
				{cache_mount} \
				'{image}:{tag}' build",
			path = self.base_path()?.display(),
			runtime_dir = runtime_dir.display(),
			image = self.image,
			tag = self.tag,
			as_root = (!self.no_root).then_some("-u root").unwrap_or_default()
		);

		log::debug!("command = {command:?}");

		let status =
			Command::new("sh").arg("-c").arg(command).stdout(std::io::stderr()).status()?;

		if !status.success() {
			bail!("Could not build runtime: srtool execution failed");
		}

		std::fs::read(format!(
			"{runtime_dir}/target/srtool/release/wbuild/{package}/{binary}.compact.compressed.wasm",
			runtime_dir = runtime_dir.display(),
			package = package,
			binary = package.replace('-', "_"),
		))
		.context("Could not load built wasm binary")
	}
}
