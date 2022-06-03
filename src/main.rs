#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![allow(dead_code)]

mod protocol;
mod commands;
mod targets;
mod miniz_types;
mod dprint;
mod serial_io;

// #[cfg(not(test))]
mod main {
    use riscv_rt::entry;
    use core::panic::PanicInfo;
    use esp32c3_hal::{ 
        Serial,
        pac,
        gpio::IO 
    };
    use crate::{ 
        protocol::Stub,
        targets::esp32c3 as target,
        dprintln,
        dprint::*,
        serial_io,
    };
    
    const MSG_BUFFER_SIZE: usize = target::MAX_WRITE_BLOCK + 0x400;

    #[entry]
    fn main() -> ! {
        let mut buffer: [u8; MSG_BUFFER_SIZE] = [0; MSG_BUFFER_SIZE];

        let peripherals = pac::Peripherals::take().unwrap();
        
        let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
        init_debug_uart(&peripherals.SYSTEM, &peripherals.UART1, io.pins.gpio10, 921600);
        
        let mut serial = Serial::new(peripherals.UART0).unwrap();
        
        // Must be called after Serial::new, as it disables interrupts
        serial_io::enable_uart0_rx_interrupt();
        
        let mut stub = Stub::new(&mut serial);
            
        stub.send_greeting();
            
        target::init();
        
        loop {
            let data = stub.read_command(&mut buffer);
            stub.process_command(data);
        }
    }

    #[panic_handler]
    fn panic(_info: &PanicInfo) -> ! {
        dprintln!("Panic !!!");
        loop {}
    }
}
