use anyhow::anyhow;

use crate::{
	cli::Toolchain,
	runtime_builder::RuntimeBuilder,
	spec::{CreateFn, PROFILES},
};

pub mod native;
pub mod srtool;

impl crate::cli::Cli {
	pub fn exec(self) -> anyhow::Result<()> {
		match self.toolchain {
			Toolchain::Native(cli) => native::exec(cli),
			Toolchain::Srtool(cli) => srtool::exec(cli),
		}
	}
}

fn build(builder: &impl RuntimeBuilder, profile: &str, raw: bool) -> anyhow::Result<()> {
	if let Some(create_fn) = fetch_create_fn(profile) {
		log::info!("Found supported profile: {}", profile);

		let chain_spec = (create_fn)(builder)?.as_json(raw).map_err(|e| anyhow!(e))?;
		print!("{chain_spec}");
		Ok(())
	} else {
		Err(anyhow!("Unknown profile, only supported: {}", supported_profile_ids()))
	}
}

fn fetch_create_fn(profile: &str) -> Option<CreateFn> {
	PROFILES.into_iter().find(|p| p.name == profile).map(|p| p.create_fn)
}

fn supported_profile_ids() -> String {
	let n = PROFILES.into_iter().count();

	PROFILES
		.into_iter()
		.map(|p| p.name)
		.enumerate()
		.fold(String::new(), |head, (i, name)| {
			let tail = if i + 1 < n { ", " } else { "" };
			format!("{head}{name}{tail}")
		})
}
