//! Mosaic Testnet Node CLI.
#![warn(missing_docs)]

#[macro_use]
mod service;
mod benchmarking;
mod cli;
mod command;
mod rpc;

#[allow(clippy::result_large_err)]
fn main() -> sdk::sc_cli::Result<()> {
	command::run()
}
