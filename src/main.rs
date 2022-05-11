#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![allow(dead_code)]

mod protocol;
mod commands;
mod targets;
mod miniz_types;

#[cfg(not(test))]
mod main {
    
    use riscv_rt::entry;
    use core::panic::PanicInfo;
    use crate::protocol::{InputIO, ErrorIO};
    use embedded_hal::serial::Read;
    use esp32c3_hal::{ Serial, pac };
    use esp_hal_common::serial::Instance;
    use crate::protocol::Stub;
    use crate::targets::esp32c3 as target;
    use nb;

    impl<'a, T: Instance> InputIO for Serial<T> {
        fn recv(&mut self) -> Result<u8, ErrorIO> {
            nb::block!(self.read()).map_err(|_| ErrorIO::Hardware)
        }

        fn send(&mut self, bytes: &[u8]) -> Result<(), ErrorIO>
        {
            self.write_bytes(bytes).map_err(|_| ErrorIO::Hardware)
        }
    }

    const MSG_BUFFER_SIZE: usize = 0x5000;

    #[entry]
    fn main() -> ! {
        let peripherals = pac::Peripherals::take().unwrap();
        
        let mut serial = Serial::new(peripherals.UART0).unwrap();

        let mut stub = Stub::new(&mut serial);

        stub.send_greeting().unwrap();

        target::init().unwrap();

        let mut buffer: [u8; MSG_BUFFER_SIZE] = [0; MSG_BUFFER_SIZE];

        loop {
            let data = stub.read_command(&mut buffer).unwrap();
            stub.process_command(data).unwrap();
        }
    }

    #[panic_handler]
    fn panic(_info: &PanicInfo) -> ! {
        loop {}
    }
}

