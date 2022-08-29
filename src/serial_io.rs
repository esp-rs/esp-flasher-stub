use heapless::spsc::Queue;
use esp_hal_common::{ 
    pac::{self, UART0},
    Serial,
    serial::Instance, 
    interrupt,
    interrupt::CpuInterrupt::*,
    Cpu::*,
    prelude::*,
};
use crate::protocol::InputIO;
use crate::targets::esp32c3 as target;

const RX_QUEUE_SIZE: usize = target::MAX_WRITE_BLOCK + 0x400;
static mut RX_QUEUE: Queue<u8, RX_QUEUE_SIZE> = Queue::new();

impl<'a, T: Instance> InputIO for Serial<T> {
    fn recv(&mut self) -> u8 {
        let mut consumer = unsafe { RX_QUEUE.split().1 };

        unsafe{
            while consumer.ready() == false { }
            consumer.dequeue_unchecked()
        }
    }

    fn send(&mut self, bytes: &[u8]) {
        self.write_bytes(bytes).unwrap()
    }
}
    
#[interrupt]
fn UART0() {
    let uart = unsafe{ &*UART0::ptr() };
    let mut producer = unsafe { RX_QUEUE.split().0 };
    
    while uart.status.read().rxfifo_cnt().bits() > 0 {
        let data = uart.fifo.read().rxfifo_rd_byte().bits();
        unsafe{ producer.enqueue_unchecked(data) };
    }
    
    uart.int_clr.write(|w| w.rxfifo_full_int_clr().set_bit());
    interrupt::clear(ProCpu, Interrupt3);
}