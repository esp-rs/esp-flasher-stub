use heapless::Deque;
#[cfg(any(target_arch = "riscv32"))]
use riscv::interrupt::free as interrupt_free;
#[cfg(any(target_arch = "xtensa"))]
use xtensa_lx::interrupt::free as interrupt_free;

use crate::{
    hal::{
        interrupt,
        interrupt::CpuInterrupt::*,
        pac,
        pac::UART0,
        prelude::*,
        serial::Instance,
        Cpu::*,
        Serial,
    },
    protocol::InputIO,
};

const RX_QUEUE_SIZE: usize = crate::targets::MAX_WRITE_BLOCK + 0x400;

static mut RX_QUEUE: Deque<u8, RX_QUEUE_SIZE> = Deque::new();

impl<'a, T: Instance> InputIO for Serial<T> {
    fn recv(&mut self) -> u8 {
        unsafe { while interrupt_free(|_| RX_QUEUE.is_empty()) {} }
        unsafe { interrupt_free(|_| RX_QUEUE.pop_front().unwrap()) }
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

#[cfg(feature = "esp32")]
pub fn enable_uart0_rx_interrupt() {
    let uart = unsafe { &*UART0::ptr() };

    uart.conf1
        .modify(|_, w| unsafe { w.rxfifo_full_thrhd().bits(1) });
    uart.int_ena.write(|w| w.rxfifo_full_int_ena().set_bit());

    unsafe {
        interrupt::map(ProCpu, pac::Interrupt::UART0, Interrupt17LevelPriority1);
        xtensa_lx::interrupt::enable_mask(1 << 17);
    }
}

#[cfg(feature = "esp32s3")]
pub fn enable_uart0_rx_interrupt() {
    let uart = unsafe { &*UART0::ptr() };

    uart.conf1
        .modify(|_, w| unsafe { w.rxfifo_full_thrhd().bits(1) });
    uart.int_ena.write(|w| w.rxfifo_full_int_ena().set_bit());

    unsafe {
        interrupt::map(ProCpu, pac::Interrupt::UART0, Interrupt17LevelPriority1);
        xtensa_lx::interrupt::enable_mask(1 << 17);
    }
}

#[cfg(feature = "esp32c3")]
pub fn enable_uart0_rx_interrupt() {
    let uart = unsafe { &*UART0::ptr() };

    uart.conf1
        .modify(|_, w| unsafe { w.rxfifo_full_thrhd().bits(1) });
    uart.int_ena.write(|w| w.rxfifo_full_int_ena().set_bit());

    interrupt::enable(pac::Interrupt::UART0, interrupt::Priority::Priority1).unwrap();
    interrupt::set_kind(ProCpu, Interrupt1, interrupt::InterruptKind::Level);

    unsafe {
        riscv::interrupt::enable();
    }
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
