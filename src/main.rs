#![no_main]
#![no_std]

#[cfg(feature = "dprint")]
use flasher_stub::hal::gpio::IO;
use flasher_stub::{
    dprintln,
    hal::{
        clock::{ClockControl, Clocks},
        entry, gpio,
        peripherals::{self, Peripherals},
        prelude::*,
        uart::{
            config::{Config, DataBits, Parity, StopBits},
            ClockSource, TxRxPins, Uart,
        },
        Blocking,
    },
    protocol::Stub,
    targets, Transport, TransportMethod,
};
use static_cell::StaticCell;

const MSG_BUFFER_SIZE: usize = targets::MAX_WRITE_BLOCK + 0x400;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    dprintln!("STUB Panic: {:?}", _info);
    loop {}
}

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();

    // If the `dprint` feature is enabled, configure/initialize the debug console,
    // which prints via UART1:

    #[cfg(feature = "dprint")]
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    #[cfg(feature = "dprint")]
    let _ = Uart::new_with_config(
        peripherals.UART1,
        Config::default(),
        Some(TxRxPins::new_tx_rx(
            io.pins.gpio2.into_push_pull_output(),
            io.pins.gpio0.into_floating_input(),
        )),
        &clocks,
        None,
    );

    // Detect the transport method being used, and configure/initialize the
    // corresponding peripheral:

    let transport = TransportMethod::detect();
    dprintln!("Stub init! Transport detected: {:?}", transport);

    let transport = match transport {
        TransportMethod::Uart => transport_uart(peripherals.UART0, &clocks),
        #[cfg(usb_device)]
        TransportMethod::UsbSerialJtag => transport_usb_serial_jtag(peripherals.USB_DEVICE),
        #[cfg(usb0)]
        TransportMethod::UsbOtg => unimplemented!(),
    };

    // With the transport initialized we can move on to initializing the stub
    // itself:

    let mut stub = Stub::new(transport);
    dprintln!("Stub sending greeting!");
    stub.send_greeting();

    // With the stub initialized and the greeting sent, all that's left to do is to
    // wait for commands to process:

    let mut buffer: [u8; MSG_BUFFER_SIZE] = [0; MSG_BUFFER_SIZE];
    loop {
        dprintln!("Waiting for command");
        let data = stub.read_command(&mut buffer);
        dprintln!("Processing command");
        stub.process_command(data);
    }
}

// Initialize the UART0 peripheral as the `Transport`.
fn transport_uart(uart0: peripherals::UART0, clocks: &Clocks<'_>) -> Transport {
    let uart_config = Config {
        baudrate: 115200,
        data_bits: DataBits::DataBits8,
        parity: Parity::ParityNone,
        stop_bits: StopBits::STOP1,
        #[cfg(not(any(feature = "esp32", feature = "esp32s2")))]
        clock_source: ClockSource::Xtal,
        #[cfg(any(feature = "esp32", feature = "esp32s2"))]
        clock_source: ClockSource::Apb,
    };

    let mut serial = Uart::new_with_config(
        uart0,
        uart_config,
        None::<TxRxPins<gpio::NoPinType, gpio::NoPinType>>,
        clocks,
        Some(flasher_stub::io::uart::uart0_handler),
    );

    serial.listen_rx_fifo_full();

    static mut TRANSPORT: StaticCell<Uart<'static, peripherals::UART0, Blocking>> =
        StaticCell::new();

    Transport::Uart(unsafe { TRANSPORT.init(serial) })
}

// Initialize the USB Serial JTAG peripheral as the `Transport`.
#[cfg(usb_device)]
fn transport_usb_serial_jtag(usb_device: peripherals::USB_DEVICE) -> Transport {
    use flasher_stub::hal::usb_serial_jtag::UsbSerialJtag;

    let mut usb_serial = UsbSerialJtag::new(
        usb_device,
        Some(flasher_stub::io::usb_serial_jtag::usb_device_handler),
    );
    usb_serial.listen_rx_packet_recv_interrupt();

    static mut TRANSPORT: StaticCell<UsbSerialJtag<'static, Blocking>> = StaticCell::new();

    Transport::UsbSerialJtag(unsafe { TRANSPORT.init(usb_serial) })
}
