#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![allow(dead_code)]

mod protocol;
mod commands;
mod targets;

// #[cfg(not(test))]
mod main {
    
    #[allow(unused)]
    extern "C" {
        fn ets_get_apb_freq() -> u32;
        fn ets_get_cpu_frequency() -> u32;
        fn ets_efuse_get_spiconfig() -> u32;
    }
    
    use riscv_rt::entry;
    // use core::fmt::Write;
    use core::panic::PanicInfo;
    use crate::protocol::{InputIO, ErrorIO};
    use embedded_hal::serial::Read;
    use nb;
    use heapless;

    use esp32c3_hal::{
        // clock_control::{sleep, ClockControl, XTAL_FREQUENCY_AUTO},
        // dprintln,
        Serial,
        pac,
        // pac::system,
        // pac::uart0,
        // Delay,
    };
    use esp_hal_common::serial::Instance;
    use crate::protocol::stub::Stub;
    use crate::targets::esp32c3 as target;

    // fn init_uart(system: &system::RegisterBlock, uart: &uart0::RegisterBlock) {
    //     system.perip_clk_en0.modify(|_, w| w.uart_mem_clk_en().set_bit()
    //                                         .uart_clk_en().set_bit());
    //     system.perip_rst_en0.modify(|_, w| w.uart1_rst().clear_bit());
    //     system.perip_rst_en0.modify(|_, w| w.uart1_rst().clear_bit());
    //     system.perip_rst_en0.modify(|_, w| w.uart1_rst().clear_bit());

    //     uart.clk_conf.modify(|_, w| w.rst_core().set_bit());
    //     system.perip_rst_en0.modify(|_, w| w.uart1_rst().set_bit());
    //     system.perip_rst_en0.modify(|_, w| w.uart1_rst().clear_bit());
    //     uart.clk_conf.modify(|_, w| w.rst_core().clear_bit());
    //     uart.id.modify(|_, w| w.reg_update().clear_bit());
        
    //     while uart.id.read().reg_update().bit_is_set() { }
    //     uart.clk_conf.modify(|_, w| unsafe{ w.sclk_sel().bits(1).sclk_div_num().bits(10) } );
    //     uart.clkdiv.modify(|_, w| unsafe{ w.clkdiv().bits(138).frag().bits(13) } );
    //     uart.id.modify(|_, w| w.reg_update().set_bit());
    // }

    struct StubIO<'a, T> {
        io: &'a mut Serial<T>
    }

    impl<'a, T> StubIO<'a, T> {
        pub fn new(serial: &'a mut Serial<T>) -> Self {
            StubIO { 
                io: serial,
            }
        }
    }

    impl<'a, T: Instance> InputIO for StubIO<'a, T> {
        fn read(&mut self) -> Result<u8, ErrorIO> {
            nb::block!(self.io.read()).map_err(|_| ErrorIO::Hardware)
        }

        fn write(&mut self, bytes: &[u8]) -> Result<(), ErrorIO>
        {
            self.io.write_bytes(bytes).map_err(|_| ErrorIO::Hardware)
        }
    }

    #[entry]
    fn main() -> ! {
        let peripherals = pac::Peripherals::take().unwrap();
        
        // let delay = Delay::new(peripherals.SYSTIMER);
        // init_uart(&peripherals.SYSTEM, &peripherals.UART1);
        
        let mut serial = Serial::new(peripherals.UART0).unwrap();

        let mut serial_io = StubIO::new(&mut serial);

        let mut stub = Stub::new(&mut serial_io);

        let mut spiconfig = unsafe{ ets_efuse_get_spiconfig() };

        let strapping = target::read_gpio_strap_reg();

        if spiconfig == 0 && (strapping & 0x1c) == 0x08 {
            spiconfig = 1; /* HSPI flash mode */
        }

        target::spi_attach(spiconfig);

        target::spi_set_default_params().unwrap();

        let mut buffer = heapless::Vec::<u8, 0x5000>::new();

        loop {
            stub.read_command(&mut buffer).unwrap();
            stub.process_command(&buffer).unwrap();

            // delay.delay(500u32 * 1000);
            // serial.write_str("Hello world\n").unwrap();
            // let apb_freq  = unsafe{ ets_get_apb_freq() };
            // let cpu_freq  = unsafe{ ets_get_cpu_frequency() };
            // writeln!(serial, "APB: {}, CPU: {}", apb_freq, cpu_freq / 2).unwrap();
            // writeln!(tx, "Characters received:  {:?}", rx.count()).unwrap();
        }
    }

    #[panic_handler]
    fn panic(_info: &PanicInfo) -> ! {
        // dprintln!("\n\n*** {:?}", info);
        loop {}
    }
}

// ~/esp/esptool.py -p /dev/ttyUSB0 --chip esp32c3  write_flash --flash_mode dio --flash_size detect --flash_freq 40m 0x0 build/bootloader/bootloader.bin 0x8000 build/partition_table/partition-table.bin 0x10000 build/hello_world.bin
