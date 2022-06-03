use heapless::Deque;
use esp32c3_hal::{ 
    Serial,
    pac,
    pac::UART0,
};
use esp_hal_common::{ 
    serial::Instance, 
    interrupt,
    interrupt::*,
    interrupt::CpuInterrupt::*,
    Cpu::*,
};
use crate::protocol::InputIO;
use crate::targets::esp32c3 as target;

const RX_QUEUE_SIZE: usize = target::MAX_WRITE_BLOCK + 0x400;
static mut RX_QUEUE: Deque<u8, RX_QUEUE_SIZE> = Deque::new();

impl<'a, T: Instance> InputIO for Serial<T> {
    fn recv(&mut self) -> u8 {
        unsafe{
            while riscv::interrupt::free(|_| RX_QUEUE.is_empty() ) { }
            riscv::interrupt::free(|_| RX_QUEUE.pop_front().unwrap_unchecked())
        }
    }

    fn send(&mut self, bytes: &[u8]) {
        self.write_bytes(bytes).unwrap()
    }
}

pub fn enable_uart0_rx_interrupt() {
    let uart = unsafe{ &*UART0::ptr() };

    uart.conf1.modify(|_, w| unsafe{ w.rxfifo_full_thrhd().bits(1) });
    uart.int_ena.write(|w| w.rxfifo_full_int_ena().set_bit() );

    interrupt::enable( ProCpu, pac::Interrupt::UART0, Interrupt3 );
    interrupt::set_kind( ProCpu, Interrupt3, InterruptKind::Level );
    interrupt::set_priority( ProCpu, Interrupt3, Priority::Priority10 );
    
    unsafe { riscv::interrupt::enable(); }
}

    
#[no_mangle]
pub fn interrupt3() {
    let uart = unsafe{ &*UART0::ptr() };
    
    while uart.status.read().rxfifo_cnt().bits() > 0 {
        let data = uart.fifo.read().rxfifo_rd_byte().bits();
        unsafe{ RX_QUEUE.push_back(data).unwrap() };
    }
    
    uart.int_clr.write(|w| w.rxfifo_full_int_clr().set_bit());
    interrupt::clear(ProCpu, Interrupt3);
}