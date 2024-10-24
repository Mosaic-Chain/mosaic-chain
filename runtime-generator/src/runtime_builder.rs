mod native;
mod srtool;

pub use native::Builder as NativeBuilder;
pub use srtool::Builder as SrtoolBuilder;

pub trait RuntimeBuilder {
	fn build(&self, package: &str, build_opts: Option<&str>) -> anyhow::Result<Vec<u8>>;
}
