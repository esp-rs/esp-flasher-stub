#![no_std]

// Re-export the correct HAL based on which feature is active
#[cfg(feature = "esp32")]
pub use esp32_hal as hal;
#[cfg(feature = "esp32c3")]
pub use esp32c3_hal as hal;
#[cfg(feature = "esp32s3")]
pub use esp32s3_hal as hal;
// Due to a bug in esp-hal this MUST be included in the root.
#[cfg(feature = "esp32c3")]
pub use hal::interrupt;
// Re-export the correct target based on which feature is active
#[cfg(feature = "esp32")]
pub use targets::Esp32 as target;
#[cfg(feature = "esp32c3")]
pub use targets::Esp32c3 as target;
#[cfg(feature = "esp32s3")]
pub use targets::Esp32s3 as target;

pub mod commands;
pub mod dprint;
pub mod miniz_types;
pub mod protocol;
pub mod serial_io;
pub mod targets;
