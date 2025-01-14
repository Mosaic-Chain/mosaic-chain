use crate::BlockNumber;

// NOTE: Currently it is not possible to change the slot duration after the chain has started.
//       Attempting to do so will brick block production.
pub const SLOT_DURATION: u64 = 6000;

pub const MINUTES: BlockNumber = 60_000 / (SLOT_DURATION as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;
pub const YEARS: BlockNumber = 5_259_600;
