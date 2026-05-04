use sdk::substrate_build_script_utils::{generate_cargo_keys, rerun_if_git_head_changed};

#[cfg(feature = "dev-spec")]
use runtime_generator::{runtime_builder::NativeBuilder, spec::para_chain::local_config};

fn main() {
	println!("cargo::rustc-check-cfg=cfg(dev_spec_built)");

	generate_cargo_keys();

	rerun_if_git_head_changed();

	if std::env::var_os("SKIP_WASM_BUILD").is_some() {
		return;
	}

	#[cfg(feature = "dev-spec")]
	{
		let inherited_features = [
			#[cfg(feature = "runtime-benchmarks")]
			"-F runtime-benchmarks ",
			#[cfg(feature = "try-runtime")]
			"-F try-runtime ",
			"",
		]
		.into_iter()
		.collect();

		println!("cargo:rustc-cfg=dev_spec_built");

		let builder = NativeBuilder {
			path: ".".into(),
			target_dir: "./target/runtime-generator".into(),
			extra_build_opts: Some(inherited_features),
			override_build_opts: None,
			out_wasm: None,
		};

		let chain_spec = local_config(&builder)
			.expect("could build wasm")
			.as_json(true)
			.expect("could create json");

		let out_dir = std::env::var("OUT_DIR").unwrap();
		let dest_path = std::path::Path::new(&out_dir).join("dev-spec.json");

		std::fs::write(&dest_path, chain_spec).unwrap();
	}
}
