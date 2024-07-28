use std::process::Command;

fn main() {
	let output = Command::new("git")
		.args(["describe", "--always", "--dirty", "--abbrev=32"])
		.output()
		.expect("Failed to execute git command");

	let git_hash = String::from_utf8_lossy(&output.stdout);
	let git_hash = git_hash.trim();
	println!("cargo:rustc-env=GIT_STATE_HASH={git_hash}");
}
