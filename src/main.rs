#![no_main]
#![no_std]

#[cfg(feature = "dprint")]
use flasher_stub::hal::uart::{
    config::{Config, DataBits, Parity, StopBits},
    TxRxPins,
};
use flasher_stub::{
    dprintln,
    hal::{
        clock::ClockControl,
        entry,
        interrupt::{self, Priority},
        peripherals::{self, Interrupt, Peripherals},
        prelude::*,
        Uart,
    },
    io::{self, Noop},
    protocol::Stub,
    targets,
    TransportMethod,
};
use static_cell::StaticCell;

const MSG_BUFFER_SIZE: usize = targets::MAX_WRITE_BLOCK + 0x400;

// TODO this sucks, but default generic parameters are not used when inference
// fails, meaning that we _have_ to specifiy the types here Seems like work on this has stalled: https://github.com/rust-lang/rust/issues/27336, note that I tried the feature and it didn't work.
#[cfg(not(any(usb_device, usb0)))]
type Transport = io::Transport<&'static mut Uart<'static, crate::peripherals::UART0>, Noop, Noop>;

#[cfg(all(usb_device, not(usb0)))]
type Transport = io::Transport<
    &'static mut Uart<'static, crate::peripherals::UART0>,
    &'static mut flasher_stub::hal::UsbSerialJtag<'static>,
    Noop,
>;

#[cfg(all(not(usb_device), usb0))]
type Transport = io::Transport<&'static mut Uart<'static, crate::peripherals::UART0>, Noop, Noop>; // TODO replace Noop with usb type later

#[cfg(all(usb_device, usb0))]
type Transport = io::Transport<
    &'static mut Uart<'static, crate::peripherals::UART0>,
    &'static mut flasher_stub::hal::UsbSerialJtag<'static>,
    Noop, // TODO replace Noop with usb type later
>;

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

    #[cfg(feature = "dprint")]
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
    );

    let transport = flasher_stub::detect_transport();
    dprintln!("Stub init! Transport detected: {:?}", transport);

    let transport = match transport {
        TransportMethod::Uart => {
            let mut serial = Uart::new(peripherals.UART0, &clocks);
            serial.listen_rx_fifo_full();
            interrupt::enable(Interrupt::UART0, Priority::Priority1).unwrap();

            static mut TRANSPORT: StaticCell<Uart<'static, crate::peripherals::UART0>> =
                StaticCell::new();

            Transport::Uart(unsafe { TRANSPORT.init(serial) })
        }
        #[cfg(usb_device)]
        TransportMethod::UsbSerialJtag => {
            let mut usb_serial = flasher_stub::hal::UsbSerialJtag::new(peripherals.USB_DEVICE);
            usb_serial.listen_rx_packet_recv_interrupt();
            interrupt::enable(Interrupt::USB_DEVICE, Priority::Priority1).unwrap();

            static mut TRANSPORT: StaticCell<flasher_stub::hal::UsbSerialJtag<'static>> =
                StaticCell::new();

            Transport::UsbSerialJtag(unsafe { TRANSPORT.init(usb_serial) })
        }
        #[cfg(usb0)]
        TransportMethod::UsbOtg => unimplemented!(),
    };

    let mut stub = Stub::new(transport);
    dprintln!("Stub sending greeting!");
    stub.send_greeting();

    let mut buffer: [u8; MSG_BUFFER_SIZE] = [0; MSG_BUFFER_SIZE];
    loop {
        dprintln!("Waiting for command");
        let data = stub.read_command(&mut buffer);
        dprintln!("Processing command");
        stub.process_command(data);
    }
}
