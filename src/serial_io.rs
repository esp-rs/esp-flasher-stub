use heapless::Deque;

use crate::{
    hal::{
        interrupt, interrupt::CpuInterrupt::*, pac, pac::UART0, prelude::*, serial::Instance,
        Cpu::*, Serial,
    },
    protocol::InputIO,
};

const RX_QUEUE_SIZE: usize = crate::targets::MAX_WRITE_BLOCK + 0x400;

static mut RX_QUEUE: Deque<u8, RX_QUEUE_SIZE> = Deque::new();

impl<'a, T: Instance> InputIO for Serial<T> {
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
        let data = uart.fifo.read().rxfifo_rd_byte().bits();
        unsafe { RX_QUEUE.push_back(data).unwrap() };
    }

    uart.int_clr.write(|w| w.rxfifo_full_int_clr().set_bit());
}

#[interrupt]
#[cfg(feature = "esp32")]
fn UART0() {
    uart_isr();
    interrupt::clear(ProCpu, Interrupt17LevelPriority1);
}

#[interrupt]
#[cfg(feature = "esp32s3")]
fn UART0() {
    uart_isr();
    interrupt::clear(ProCpu, Interrupt17LevelPriority1);
}

#[interrupt]
#[cfg(feature = "esp32c3")]
fn UART0() {
    uart_isr();
    interrupt::clear(ProCpu, Interrupt1);
}
