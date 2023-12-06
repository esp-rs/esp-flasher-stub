#![no_std]

// Re-export the correct HAL based on which feature is active
#[cfg(feature = "esp32")]
pub use esp32_hal as hal;
#[cfg(feature = "esp32c2")]
pub use esp32c2_hal as hal;
#[cfg(feature = "esp32c3")]
pub use esp32c3_hal as hal;
#[cfg(feature = "esp32c6")]
pub use esp32c6_hal as hal;
#[cfg(feature = "esp32h2")]
pub use esp32h2_hal as hal;
#[cfg(feature = "esp32s2")]
pub use esp32s2_hal as hal;
#[cfg(feature = "esp32s3")]
pub use esp32s3_hal as hal;
// Re-export the correct target based on which feature is active
#[cfg(feature = "esp32")]
pub use targets::Esp32 as target;
#[cfg(feature = "esp32c2")]
pub use targets::Esp32c2 as target;
#[cfg(feature = "esp32c3")]
pub use targets::Esp32c3 as target;
#[cfg(feature = "esp32c6")]
pub use targets::Esp32c6 as target;
#[cfg(feature = "esp32h2")]
pub use targets::Esp32h2 as target;
#[cfg(feature = "esp32s2")]
pub use targets::Esp32s2 as target;
#[cfg(feature = "esp32s3")]
pub use targets::Esp32s3 as target;

pub mod commands;
pub mod dprint;
pub mod io;
pub mod miniz_types;
pub mod protocol;
pub mod targets;

#[derive(Debug)]
pub enum TransportMethod {
    Uart,
    #[cfg(usb_device)]
    UsbSerialJtag,
    #[cfg(usb0)]
    UsbOtg,
}

pub fn detect_transport() -> TransportMethod {
    #[cfg(usb0)]
    use crate::targets::EspUsbOtgId as _;
    #[cfg(usb_device)]
    use crate::targets::EspUsbSerialJtagId as _;

    #[repr(C)]
    struct Uart {
        baud_rate: u32,
        data_bits: u32,
        exist_parity: u32,
        parity: u32,
        stop_bits: u32,
        flow_ctrl: u32,
        buff_uart_no: u8,
        rcv_buff: [u32; 2], // PAD
        rcv_state: u32,
        received: u32,
    }

    extern "C" {
        fn esp_flasher_rom_get_uart() -> *const Uart;
    }

    let device = unsafe { esp_flasher_rom_get_uart() };
    let num = unsafe { (*device).buff_uart_no };
    match num {
        #[cfg(usb_device)]
        target::USB_SERIAL_JTAG_ID => TransportMethod::UsbSerialJtag,
        #[cfg(usb0)]
        target::USB_OTG_ID => TransportMethod::UsbOtg,
        _ => TransportMethod::Uart,
    }
}
