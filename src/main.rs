#![no_main]
#![no_std]

use esp_backtrace as _;
use flasher_stub::{
    hal::{
        clock::ClockControl,
        pac,
        prelude::*,
        serial::{
            config::{Config, DataBits, Parity, StopBits},
            TxRxPins,
        },
        Serial,
        IO,
    },
    protocol::Stub,
    targets,
    serial_io,
};

#[cfg(target_arch = "riscv32")]
use riscv_rt::entry;
#[cfg(target_arch = "xtensa")]
use xtensa_lx_rt::entry;

const MSG_BUFFER_SIZE: usize = targets::MAX_WRITE_BLOCK + 0x400;

#[entry]
fn main() -> ! {
    let peripherals = pac::Peripherals::take().unwrap();
    #[cfg(any(feature = "esp32c3", feature = "esp32s3"))]
    let system = peripherals.SYSTEM.split();
    #[cfg(any(feature = "esp32"))]
    let system = peripherals.DPORT.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let pins = TxRxPins::new_tx_rx(
        io.pins.gpio2.into_push_pull_output(),
        io.pins.gpio0.into_floating_input(),
    );

    let config = Config {
        baudrate: 115200,
        data_bits: DataBits::DataBits8,
        parity: Parity::ParityNone,
        stop_bits: StopBits::STOP1,
    };

    let _ = Serial::new_with_config(peripherals.UART1, Some(config), Some(pins), &clocks);

    let mut serial = Serial::new(peripherals.UART0);

    // Must be called after Serial::new, as it disables interrupts
    serial_io::enable_uart0_rx_interrupt();

    let mut stub = Stub::new(&mut serial);
    stub.send_greeting();

    let mut buffer: [u8; MSG_BUFFER_SIZE] = [0; MSG_BUFFER_SIZE];
    loop {
        let data = stub.read_command(&mut buffer);
        stub.process_command(data);
    }
}
