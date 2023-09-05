use core::marker::PhantomData;

use heapless::Deque;

use crate::protocol::InputIO;

pub mod uart;
#[cfg(any(
    feature = "esp32c3",
    feature = "esp32s3",
    feature = "esp32c6",
    feature = "esp32h2"
))]
pub mod usb_serial_jtag;

const RX_QUEUE_SIZE: usize = crate::targets::MAX_WRITE_BLOCK + 0x400;

static mut RX_QUEUE: Deque<u8, RX_QUEUE_SIZE> = Deque::new();

#[non_exhaustive]
pub enum Transport<S, J> {
    Uart(S),
    #[cfg(any(
        feature = "esp32c3",
        feature = "esp32s3",
        feature = "esp32c6",
        feature = "esp32h2"
    ))]
    UsbSerialJtag(J),
    // #[cfg(any(feature = "esp32s2", feature = "esp32s3"))]
    // UsbOtg(U),
    #[doc(hidden)] // a type to "use" the generic params
    __Hidden(PhantomData<S>, PhantomData<J> /* , PhantomData<U> */),
}

impl<S, J> InputIO for Transport<S, J>
where
    S: InputIO,
    J: InputIO,
{
    fn recv(&mut self) -> u8 {
        match self {
            Transport::Uart(s) => s.recv(),
            #[cfg(any(
                feature = "esp32c3",
                feature = "esp32s3",
                feature = "esp32c6",
                feature = "esp32h2"
            ))]
            Transport::UsbSerialJtag(j) => j.recv(),
            _ => unreachable!(),
        }
    }

    fn send(&mut self, data: &[u8]) {
        match self {
            Transport::Uart(s) => s.send(data),
            #[cfg(any(
                feature = "esp32c3",
                feature = "esp32s3",
                feature = "esp32c6",
                feature = "esp32h2"
            ))]
            Transport::UsbSerialJtag(j) => j.send(data),
            _ => unreachable!(),
        }
    }
}
