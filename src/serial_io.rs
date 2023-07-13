use heapless::Deque;

use crate::{
    hal::{
        interrupt,
        interrupt::CpuInterrupt::*,
        peripherals::UART0,
        prelude::*,
        uart::Instance,
        Cpu::*,
        Uart,
    },
    protocol::InputIO,
};

const RX_QUEUE_SIZE: usize = crate::targets::MAX_WRITE_BLOCK + 0x400;

static mut RX_QUEUE: Deque<u8, RX_QUEUE_SIZE> = Deque::new();

impl<T: Instance> InputIO for Uart<'_, T> {
    fn recv(&mut self) -> u8 {
        unsafe { while critical_section::with(|_| RX_QUEUE.is_empty()) {} }
        unsafe { critical_section::with(|_| RX_QUEUE.pop_front().unwrap()) }
    }

    fn send(&mut self, bytes: &[u8]) {
        self.write_bytes(bytes).unwrap()
    }
}

fn uart_isr() {
    let uart = unsafe { &*UART0::ptr() };

    while uart.status.read().rxfifo_cnt().bits() > 0 {
        let offset = if cfg!(feature = "esp32s2") {
            0x20C0_0000
        } else {
            0
        };

        // read a bye from the fifo
        let data = unsafe { (uart.fifo.as_ptr() as *mut u8).offset(offset).read() };

        unsafe { RX_QUEUE.push_back(data).unwrap() };
    }

    uart.int_clr.write(|w| w.rxfifo_full_int_clr().set_bit());
}

#[interrupt]
fn UART0() {
    uart_isr();
    #[cfg(any(feature = "esp32", feature = "esp32s2", feature = "esp32s3"))]
    interrupt::clear(ProCpu, Interrupt17LevelPriority1);
    #[cfg(any(feature = "esp32c3", feature = "esp32c2"))]
    interrupt::clear(ProCpu, Interrupt1);
}
