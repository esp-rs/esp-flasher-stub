#![no_std]

// Re-export the correct target based on which feature is active
#[cfg(feature = "esp32")]
pub use targets::Esp32 as Target;
#[cfg(feature = "esp32c2")]
pub use targets::Esp32c2 as Target;
#[cfg(feature = "esp32c3")]
pub use targets::Esp32c3 as Target;
#[cfg(feature = "esp32c6")]
pub use targets::Esp32c6 as Target;
#[cfg(feature = "esp32h2")]
pub use targets::Esp32h2 as Target;
#[cfg(feature = "esp32s2")]
pub use targets::Esp32s2 as Target;
#[cfg(feature = "esp32s3")]
pub use targets::Esp32s3 as Target;

pub mod commands;
pub mod dprint;
pub mod io;
pub mod miniz_types;
pub mod protocol;
pub mod targets;

pub use esp_hal as hal;

use self::{
    hal::{peripherals::UART0, uart::Uart, Blocking},
    io::Noop,
};

#[derive(Debug)]
pub enum TransportMethod {
    Uart,
    #[cfg(usb_device)]
    UsbSerialJtag,
    #[cfg(usb0)]
    UsbOtg,
}

impl TransportMethod {
    pub fn detect() -> Self {
        #[cfg(usb0)]
        use crate::targets::EspUsbOtgId as _;
        #[cfg(usb_device)]
        use crate::targets::EspUsbSerialJtagId as _;

        extern "C" {
            fn esp_flasher_rom_get_uart() -> *const Uart;
        }

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

        let device = unsafe { esp_flasher_rom_get_uart() };
        let num = unsafe { (*device).buff_uart_no };

        match num {
            #[cfg(usb_device)]
            Target::USB_SERIAL_JTAG_ID => TransportMethod::UsbSerialJtag,
            #[cfg(usb0)]
            Target::USB_OTG_ID => TransportMethod::UsbOtg,
            _ => TransportMethod::Uart,
        }
    }
}

// TODO this sucks, but default generic parameters are not used when inference
// fails, meaning that we _have_ to specifiy the types here Seems like work on this has stalled: https://github.com/rust-lang/rust/issues/27336, note that I tried the feature and it didn't work.
#[cfg(not(any(usb_device, usb0)))]
pub type Transport = io::Transport<&'static mut Uart<'static, UART0, Blocking>, Noop, Noop>;

#[cfg(all(usb_device, not(usb0)))]
pub type Transport = io::Transport<
    &'static mut Uart<'static, UART0, Blocking>,
    &'static mut crate::hal::usb_serial_jtag::UsbSerialJtag<'static, Blocking>,
    Noop,
>;

#[cfg(all(not(usb_device), usb0))]
pub type Transport = io::Transport<&'static mut Uart<'static, UART0, Blocking>, Noop, Noop>; // TODO replace Noop with usb type later

#[cfg(all(usb_device, usb0))]
pub type Transport = io::Transport<
    &'static mut Uart<'static, UART0, Blocking>,
    &'static mut crate::hal::usb_serial_jtag::UsbSerialJtag<'static, Blocking>,
    Noop, // TODO replace Noop with usb type later
>;
