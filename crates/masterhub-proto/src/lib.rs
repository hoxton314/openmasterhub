//! Wire protocol for Cooler Master MasterHUB modules.
//!
//! Pure encode/decode, no I/O. Every module is its own USB HID device (VID `0x2516`) with:
//! - input report `0x01` (64 bytes incl. ID): events
//! - output report `0x02` (64 bytes, or 320 for display modules): commands and image data
//! - feature reports (32 bytes incl. ID): `0x03` for SET commands, others for GET queries
//!
//! See `docs/PROTOCOL.md` for the hardware-verified spec.

mod command;
mod event;
mod info;
mod module;

pub use command::*;
pub use event::*;
pub use info::*;
pub use module::*;

/// USB vendor ID of all MasterHUB modules.
pub const VENDOR_ID: u16 = 0x2516;
/// Size of a feature report buffer including the report ID.
pub const FEATURE_LEN: usize = 32;
/// Report ID used for feature SET commands. Display modules apply a SET only after this
/// report is read back with a GET.
pub const FEATURE_COMMAND: u8 = 0x03;
