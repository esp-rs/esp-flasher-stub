use super::{UartMarker, RX_QUEUE};
use crate::{
    hal::{peripherals::UART0, prelude::*, uart::Instance, Uart},
    protocol::InputIO, dprintln,
};

impl<T: Instance> InputIO for Uart<'_, T> {
    fn recv(&mut self) -> u8 {
        unsafe { while critical_section::with(|_| RX_QUEUE.is_empty()) {} }
        unsafe { critical_section::with(|_| RX_QUEUE.pop_front().unwrap()) }
    }

    fn send(&mut self, bytes: &[u8]) {
        self.write_bytes(bytes).unwrap();
        // crate::io::uart::nb::block!(self.flush()).unwrap();
    }
}

impl<T: Instance> UartMarker for Uart<'_, T> {}

#[interrupt]
fn UART0() {
    flush();
}

pub fn flush() {
    let uart = unsafe { &*UART0::ptr() };
    
    // dprintln!("Bytes in fifo: {}", UART0::get_rx_fifo_count());
    // while UART0::get_rx_fifo_count() > 0 {
    while uart.status.read().rxfifo_cnt().bits() > 0 {
        let offset = if cfg!(feature = "esp32s2") {
            0x20C0_0000
        } else {
            0
        };

        // read a byte from the fifo
        // the read _must_ be a word read so the hardware correctly detects the read and
        // pops the byte from the fifo cast the result to a u8, as only the
        // first byte contains the data
        let data = unsafe { uart.fifo.as_ptr().offset(offset).read() } as u8;
        unsafe { RX_QUEUE.push_back(data).unwrap() };
    }

    uart.int_clr.write(|w| w.rxfifo_full_int_clr().set_bit());
}
