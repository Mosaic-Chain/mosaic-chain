//! Mosaic Testnet Node CLI.
#![warn(missing_docs)]

#[macro_use]
mod service;
mod benchmarking;
mod cli;
mod command;
mod rpc;

fn main() -> sdk::sc_cli::Result<()> {
	command::run()
}
