#[path = "../../parachain/src/params/common_constant.rs"]
mod common_constant;
#[path = "../../parachain/src/params/currency.rs"]
pub mod currency;
#[path = "../../parachain/src/params/dynamic.rs"]
pub mod dynamic;
#[path = "../../parachain/src/params/time.rs"]
pub mod time;

pub mod constant {
	pub use super::common_constant::*;

	pub mod grandpa {
		use sdk::frame_support::parameter_types;

		parameter_types! {
			pub const MaxAuthorities: u32 = 400;
			pub const MaxSetIdSessionEntries: u64 = 0;
			// We have no such things as nominators, grandpa only uses this for weight calculation of offence reporting extrinsics.
			// Since we use the default implementation of EquivocationReportSystem (noop), we can safely set this to 0.
			pub const MaxNominators: u32 = 0;
		}
	}
}

pub use dynamic::RuntimeParameters;
