//! Print debug information to UART1
//!
//! Directly writes to the UART1 TX uart queue.
//! This is unsafe! It is asynchronous with normal UART1 usage and
//! interrupts are not disabled.

use esp32c3_hal::pac::UART1;
use esp_hal_common::{ 
    OutputPin, 
    types::OutputSignal, 
};
pub struct DebugLog {}

pub enum Error {}

use esp32c3_hal::pac::{ system, uart0 };

static mut SCLK_SEL: u8 = 0;

fn div_up(a: u32, b: u32) -> u32{
    (a + b - 1) / b
}

fn get_sclk_freq(uart: &uart0::RegisterBlock) -> u32
{
    const APB_CLK_FREQ: u32 = 80000000;
    const RTC_CLK_FREQ: u32 = 20000000;
    const XTAL_CLK_FREQ: u32 = 40000000;

    unsafe{ SCLK_SEL = uart.clk_conf.read().sclk_sel().bits(); }

    
    match uart.clk_conf.read().sclk_sel().bits() {
        1 => APB_CLK_FREQ,
        2 => RTC_CLK_FREQ,
        3 => XTAL_CLK_FREQ,
        _ => XTAL_CLK_FREQ,
    }
}

fn set_baudrate(uart: &uart0::RegisterBlock, baud: u32)
{ 
    let sclk_freq: u32 = get_sclk_freq(uart);
    let max_div: u32 = (1 << 12) - 1;
    let sclk_div: u32 = div_up(sclk_freq, max_div * baud);
    let clk_div: u32 = ((sclk_freq) << 4) / (baud * sclk_div);
    let clk_div_shift = (clk_div >> 4) as u16;
    
    uart.clkdiv.modify(|_, w| unsafe{ w.clkdiv().bits(clk_div_shift)
        .frag().bits((clk_div &  0xf) as u8) } );
    uart.clk_conf.modify(|_, w| unsafe{ w.sclk_div_num().bits((sclk_div - 1) as u8) } );
    }
    
    pub fn init_debug_uart <TxPin: OutputPin<OutputSignal = OutputSignal>>(
        system: &system::RegisterBlock, 
        uart: &uart0::RegisterBlock,
        mut tx_pin: TxPin,
        baudrate: u32) {
    
    tx_pin.set_to_push_pull_output().connect_peripheral_to_output(OutputSignal::U1TXD);

    system.perip_clk_en0.modify(|_, w| w.uart_mem_clk_en().set_bit()
    .uart_clk_en().set_bit());
    system.perip_rst_en0.modify(|_, w| w.uart1_rst().clear_bit());
    
    uart.clk_conf.modify(|_, w| w.rst_core().set_bit());
    system.perip_rst_en0.modify(|_, w| w.uart1_rst().set_bit());
    system.perip_rst_en0.modify(|_, w| w.uart1_rst().clear_bit());
    uart.clk_conf.modify(|_, w| w.rst_core().clear_bit());
    uart.id.modify(|_, w| w.reg_update().clear_bit());
    
    while uart.id.read().reg_update().bit_is_set() { }
    set_baudrate(uart, baudrate);
    uart.id.modify(|_, w| w.reg_update().set_bit());
}


impl DebugLog {
    pub fn count(&mut self) -> u16 {
        unsafe { (*UART1::ptr()).status.read().txfifo_cnt().bits() }
    }

    // pub fn is_idle(&mut self) -> bool {
    //     unsafe { (*UART1::ptr()).status.read().st_utx_out().is_tx_idle() }
    // }

    fn write(&mut self, word: u8) -> nb::Result<(), Error> {
        if self.count() < 128 {
            unsafe{ (*UART1::ptr()).fifo.write(|w| w.rxfifo_rd_byte().bits(word) ) };
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

impl core::fmt::Write for DebugLog {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        s.as_bytes()
            .iter()
            .try_for_each(|c| nb::block!(self.write(*c)))
            .map_err(|_| core::fmt::Error)
    }
}

pub static mut DEBUG_LOG: DebugLog = DebugLog {};

/// Macro for sending a formatted string to UART1 for debugging
#[macro_export]
macro_rules! dprint {
    ($s:expr) => {
        #[allow(unused_unsafe)]
        unsafe {
            use core::fmt::Write;
            $crate::dprint::DEBUG_LOG.write_str($s).unwrap();
        }
    };
    ($($arg:tt)*) => {
        #[allow(unused_unsafe)]
        unsafe {
            use core::fmt::Write;
            $crate::dprint::DEBUG_LOG.write_fmt(format_args!($($arg)*)).unwrap();
        }
    };
}

/// Macro for sending a formatted string to UART1 for debugging, with a newline.
#[macro_export]
macro_rules! dprintln {
    () => {
        #[allow(unused_unsafe)]
        unsafe {
            use core::fmt::Write;
            $crate::dprint::DEBUG_LOG.write_str("\n").unwrap();
        }
    };
    ($fmt:expr) => {
        #[allow(unused_unsafe)]
        unsafe {
            use core::fmt::Write;
            $crate::dprint::DEBUG_LOG.write_str(concat!($fmt, "\n")).unwrap();
        }
    };
    ($fmt:expr, $($arg:tt)*) => {
        #[allow(unused_unsafe)]
        unsafe {
            use core::fmt::Write;
            $crate::dprint::DEBUG_LOG.write_fmt(format_args!(concat!($fmt, "\n"), $($arg)*)).unwrap();
        }
    };
}

/// Macro for flushing the UART1 TX buffer
#[macro_export]
macro_rules! dflush {
    () => {
        #[allow(unused_unsafe)]
        unsafe {
            while !$crate::dprint::DEBUG_LOG.is_idle() {}
        }
    };
}
