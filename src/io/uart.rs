use super::{UartMarker, RX_QUEUE};
use crate::{
    hal::{macros::interrupt, peripherals::UART0, uart::Instance, Uart},
    protocol::InputIO,
};

impl<T> InputIO for Uart<'_, T>
where
    T: Instance,
{
    fn recv(&mut self) -> u8 {
        unsafe {
            while critical_section::with(|_| RX_QUEUE.is_empty()) {}
            critical_section::with(|_| RX_QUEUE.pop_front().unwrap())
        }
    }

    fn send(&mut self, bytes: &[u8]) {
        self.write_bytes(bytes).unwrap();
    }
}

impl<T> UartMarker for Uart<'_, T> where T: Instance {}

#[interrupt]
fn UART0() {
    let uart = unsafe { &*UART0::ptr() };

    while uart.status().read().rxfifo_cnt().bits() > 0 {
        let offset = if cfg!(feature = "esp32s2") {
            0x20C0_0000
        } else {
            0
        };

        // read a byte from the fifo
        // the read _must_ be a word read so the hardware correctly detects the read and
        // pops the byte from the fifo cast the result to a u8, as only the
        // first byte contains the data
        let data = unsafe { uart.fifo().as_ptr().offset(offset / 4).read() } as u8;
        unsafe { RX_QUEUE.push_back(data).unwrap() };
    }

    uart.int_clr().write(|w| w.rxfifo_full_int_clr().set_bit());
}
