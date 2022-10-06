use esp32c3_hal::{
    interrupt,
    interrupt::CpuInterrupt,
    pac::{self, UART0},
    prelude::*,
    serial::Instance,
    Cpu,
    Serial,
};
use heapless::spsc::Queue;

use crate::{protocol::InputIO, targets::esp32c3 as target};

const RX_QUEUE_SIZE: usize = target::MAX_WRITE_BLOCK + 0x400;
static mut RX_QUEUE: Queue<u8, RX_QUEUE_SIZE> = Queue::new();

impl<'a, T: Instance> InputIO for Serial<T> {
    fn recv(&mut self) -> u8 {
        loop {
            if let Some(byte) = unsafe { RX_QUEUE.dequeue() } {
                return byte;
            }
        }
    }

    fn send(&mut self, bytes: &[u8]) {
        self.write_bytes(bytes).unwrap()
    }
}

#[interrupt]
fn UART0() {
    let uart = unsafe { &*UART0::ptr() };

    while uart.status.read().rxfifo_cnt().bits() > 0 {
        let data = uart.fifo.read().rxfifo_rd_byte().bits();
        unsafe { RX_QUEUE.enqueue(data).unwrap() };
    }

    uart.int_clr.write(|w| w.rxfifo_full_int_clr().set_bit());
    interrupt::clear(Cpu::ProCpu, CpuInterrupt::Interrupt3);
}
