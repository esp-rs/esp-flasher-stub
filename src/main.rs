#![no_main]
#![no_std]

use esp_backtrace as _;
#[cfg(feature = "dprint")]
use flasher_stub::hal::serial::{
    config::{Config, DataBits, Parity, StopBits},
    TxRxPins,
};
use flasher_stub::{
    hal::{clock::ClockControl, interrupt, pac, prelude::*, Serial, IO},
    protocol::Stub,
    targets,
};
#[cfg(target_arch = "riscv32")]
use riscv_rt::entry;
#[cfg(target_arch = "xtensa")]
use xtensa_lx_rt::entry;

const MSG_BUFFER_SIZE: usize = targets::MAX_WRITE_BLOCK + 0x400;

#[entry]
fn main() -> ! {
    let peripherals = pac::Peripherals::take().unwrap();
    #[cfg(not(feature = "esp32"))]
    let system = peripherals.SYSTEM.split();
    #[cfg(feature = "esp32")]
    let system = peripherals.DPORT.split();

    #[cfg(not(feature = "esp32c2"))]
    #[allow(unused)]
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze(); // TODO: configure clocks for all chips
    #[cfg(feature = "esp32c2")]
    #[allow(unused)]
    let clocks = ClockControl::configure(
        system.clock_control,
        flasher_stub::hal::clock::CpuClock::Clock120MHz,
    )
    .freeze();

    #[allow(unused)]
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    #[cfg(feature = "dprint")]
    let _ = Serial::new_with_config(
        peripherals.UART1,
        Some(Config {
            baudrate: 115200,
            data_bits: DataBits::DataBits8,
            parity: Parity::ParityNone,
            stop_bits: StopBits::STOP1,
        }),
        Some(TxRxPins::new_tx_rx(
            io.pins.gpio2.into_push_pull_output(),
            io.pins.gpio0.into_floating_input(),
        )),
        &clocks,
    );

    let mut serial = Serial::new(peripherals.UART0);

    // Must be called after Serial::new, as it disables interrupts
    serial.listen_rx_fifo_full();

    interrupt::enable(pac::Interrupt::UART0, interrupt::Priority::Priority1).unwrap();

    let mut stub = Stub::new(&mut serial);
    stub.send_greeting();

    let mut buffer: [u8; MSG_BUFFER_SIZE] = [0; MSG_BUFFER_SIZE];
    loop {
        let data = stub.read_command(&mut buffer);
        stub.process_command(data);
    }
}
