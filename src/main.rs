#![allow(dead_code)]
#![no_main]
#![no_std]

mod protocol;
mod commands;
mod targets;
mod miniz_types;
mod dprint;
mod serial_io;

#[cfg(feature = "esp32c3")]
use esp32c3_hal::{ interrupt, IO };
#[cfg(feature = "esp32")]
use esp32_hal::{ IO };

#[cfg(any(target_arch = "riscv32"))]
use riscv_rt::entry;
#[cfg(any(target_arch = "xtensa"))]
use xtensa_lx_rt::entry;

use esp_backtrace as _;
use esp_hal_common::{
    prelude::SystemExt,
    clock::ClockControl,
    serial::config::Config,
    serial::TxRxPins,
    Serial,
    pac,
};
use crate::protocol::Stub;


#[entry]
fn main() -> ! {
    const MSG_BUFFER_SIZE: usize = crate::targets::MAX_WRITE_BLOCK + 0x400;
    let mut buffer: [u8; MSG_BUFFER_SIZE] = [0; MSG_BUFFER_SIZE];
    
    let peripherals = pac::Peripherals::take().unwrap();
    
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let pins = TxRxPins::new_tx_rx(
        io.pins.gpio18.into_push_pull_output(),
        io.pins.gpio9.into_floating_input(),
    );
    #[cfg(any(target_arch = "xtensa"))]
    let system = peripherals.DPORT.split();
    #[cfg(any(target_arch = "riscv32"))]
    let system = peripherals.SYSTEM.split();
    let cfg = Config::default().baudrate(921600);
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();
    let _ = Serial::new_with_config(peripherals.UART1, Some(cfg), Some(pins), &clocks);
    
    let mut serial = Serial::new(peripherals.UART0);
    
    // Must be called after Serial::new, as it disables interrupts
    serial_io::enable_uart0_rx_interrupt();
    
    let mut stub = Stub::new(&mut serial);
    
    stub.send_greeting();
    
    loop {
        let data = stub.read_command(&mut buffer);
        stub.process_command(data);
    }
}
