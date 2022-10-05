#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![allow(dead_code)]

use esp32c3_hal::interrupt;

mod commands;
mod dprint;
mod miniz_types;
mod protocol;
mod serial_io;
mod targets;

// #[cfg(not(test))]
mod main {
    use riscv_rt::entry;
    use core::panic::PanicInfo;
    use crate::{ 
        protocol::Stub,
        targets::esp32c3 as target,
        dprintln,
    };

    use esp32c3_hal::IO;
    use esp_hal_common::{
        pac,
        serial::{ Serial, TxRxPins, config::Config},
        clock::ClockControl,
        prelude::SystemExt,
        interrupt,
        interrupt::*,
        interrupt::CpuInterrupt::*,
        Cpu::*,
    };

    const MSG_BUFFER_SIZE: usize = target::MAX_WRITE_BLOCK + 0x400;

    #[entry]
    fn main() -> ! {
        let mut buffer: [u8; MSG_BUFFER_SIZE] = [0; MSG_BUFFER_SIZE];

        let peripherals = pac::Peripherals::take().unwrap();
        
        // Init debug UART
        let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
        let pins = TxRxPins::new_tx_rx(
            io.pins.gpio10.into_push_pull_output(),
            io.pins.gpio9.into_floating_input(),
        );
        let system = peripherals.SYSTEM.split();
        let cfg = Config::default().baudrate(921600);
        let clocks = ClockControl::boot_defaults(system.clock_control).freeze();
        let _ = Serial::new_with_config(peripherals.UART1, Some(cfg), Some(pins), &clocks);
        
        // Init IO serial
        let mut serial = Serial::new(peripherals.UART0);
        serial.set_rx_fifo_full_threshold(1);
        serial.listen_rx_fifo_full();
        interrupt::enable( pac::Interrupt::UART0, interrupt::Priority::Priority3 ).ok();
        interrupt::set_kind( ProCpu, Interrupt3, InterruptKind::Level );
        unsafe{ interrupt::set_priority( ProCpu, Interrupt3, Priority::Priority10 ) };
        unsafe { riscv::interrupt::enable(); }
        
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
