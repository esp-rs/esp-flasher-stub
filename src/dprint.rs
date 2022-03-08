//! Print debug information to UART1
//!
//! Directly writes to the UART1 TX uart queue.
//! This is unsafe! It is asynchronous with normal UART1 usage and
//! interrupts are not disabled.

use esp_hal_common::pac::UART1;
pub struct DebugLog {}

pub enum Error {}

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
