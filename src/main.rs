#![no_main]
#![no_std]

#[cfg(feature = "dprint")]
use flasher_stub::hal::uart::{
    config::{Config, DataBits, Parity, StopBits},
    TxRxPins,
};
use flasher_stub::{
    hal::{clock::ClockControl, interrupt, peripherals, prelude::*, Uart, IO},
    io::Transport,
    protocol::Stub,
    targets,
};
use static_cell::StaticCell;

const MSG_BUFFER_SIZE: usize = targets::MAX_WRITE_BLOCK + 0x400;

#[flasher_stub::hal::entry]
fn main() -> ! {
    let peripherals = peripherals::Peripherals::take();
    #[cfg(not(any(feature = "esp32", feature = "esp32c6", feature = "esp32h2")))]
    let mut system = peripherals.SYSTEM.split();
    #[cfg(feature = "esp32")]
    let mut system = peripherals.DPORT.split();
    #[cfg(any(feature = "esp32c6", feature = "esp32h2"))]
    let mut system = peripherals.PCR.split();

    #[cfg(any(feature = "esp32", feature = "esp32s2"))]
    #[allow(unused)]
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze(); // TODO: ESP32 and S2 only works with `boot_defauls` for some reason

    #[cfg(any(
        feature = "esp32c2",
        feature = "esp32c3",
        feature = "esp32h2",
        feature = "esp32c6",
        feature = "esp32s3"
    ))]
    #[allow(unused)]
    let clocks = ClockControl::max(system.clock_control).freeze();

    #[allow(unused)]
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    #[cfg(feature = "dprint")]
    let _ = Uart::new_with_config(
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
        &mut system.peripheral_clock_control,
    );

    let transport = flasher_stub::detect_transport();
    flasher_stub::dprintln!("Stub init! Transport detected: {:?}", transport);

    let transport = match transport {
        flasher_stub::TransportMethod::Uart => {
            let mut serial = Uart::new(peripherals.UART0, &mut system.peripheral_clock_control);

            serial.listen_rx_fifo_full();

            interrupt::enable(
                peripherals::Interrupt::UART0,
                interrupt::Priority::Priority1,
            )
            .unwrap();

            static mut TRANSPORT: StaticCell<Uart<'static, crate::peripherals::UART0>> =
                StaticCell::new();
            Transport::Uart(unsafe { TRANSPORT.init(serial) })
        }
        #[cfg(any(
            feature = "esp32c3",
            feature = "esp32s3",
            feature = "esp32c6",
            feature = "esp32h2"
        ))]
        flasher_stub::TransportMethod::UsbSerialJtag => {
            let mut usb_serial = flasher_stub::hal::UsbSerialJtag::new(
                peripherals.USB_DEVICE,
                &mut system.peripheral_clock_control,
            );
            usb_serial.listen_rx_packet_recv_interrupt();
            interrupt::enable(
                peripherals::Interrupt::USB_DEVICE,
                interrupt::Priority::Priority1,
            )
            .unwrap();

            static mut TRANSPORT: StaticCell<flasher_stub::hal::UsbSerialJtag<'static>> =
                StaticCell::new();
            Transport::UsbSerialJtag(unsafe { TRANSPORT.init(usb_serial) })
        }
        #[cfg(any(feature = "esp32s2", feature = "esp32s3"))]
        flasher_stub::TransportMethod::UsbOtg => unimplemented!(),
    };

    let mut stub = Stub::new(transport);
    flasher_stub::dprintln!("Stub sending greeting!");
    stub.send_greeting();

    let mut buffer: [u8; MSG_BUFFER_SIZE] = [0; MSG_BUFFER_SIZE];
    loop {
        flasher_stub::dprintln!("Waiting for command");
        let data = stub.read_command(&mut buffer);
        flasher_stub::dprintln!("Processing command");
        stub.process_command(data);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    flasher_stub::dprintln!("STUB Panic: {:?}", _info);
    loop {}
}
