use heapless::Deque;


pub mod uart;

const RX_QUEUE_SIZE: usize = crate::targets::MAX_WRITE_BLOCK + 0x400;

static mut RX_QUEUE: Deque<u8, RX_QUEUE_SIZE> = Deque::new();