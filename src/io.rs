use core::marker::PhantomData;

use heapless::Deque;

use crate::protocol::InputIO;

pub mod uart;
#[cfg(usb_device)]
pub mod usb_serial_jtag;

const RX_QUEUE_SIZE: usize = crate::targets::MAX_WRITE_BLOCK + 0x400;

static mut RX_QUEUE: Deque<u8, RX_QUEUE_SIZE> = Deque::new();

trait UartMarker: InputIO {}
trait UsbSerialJtagMarker: InputIO {}
trait UsbOtgMarker: InputIO {}

#[non_exhaustive]
pub enum Transport<S, J, U> {
    Uart(S),
    #[cfg(usb_device)]
    UsbSerialJtag(J),
    #[cfg(usb0)]
    UsbOtg(U),
    #[doc(hidden)]
    __Hidden(PhantomData<J>, PhantomData<U>),
}

impl<S, J, U> InputIO for Transport<S, J, U>
where
    S: UartMarker,
    J: UsbSerialJtagMarker,
    U: UsbOtgMarker,
{
    fn recv(&mut self) -> u8 {
        match self {
            Transport::Uart(s) => s.recv(),
            #[cfg(usb_device)]
            Transport::UsbSerialJtag(j) => j.recv(),
            _ => todo!(),
        }
    }

    fn send(&mut self, data: &[u8]) {
        match self {
            Transport::Uart(s) => s.send(data),
            #[cfg(usb_device)]
            Transport::UsbSerialJtag(j) => j.send(data),
            _ => todo!(),
        }
    }
}

pub struct Noop;

impl InputIO for Noop {
    fn recv(&mut self) -> u8 {
        todo!()
    }

    fn send(&mut self, _data: &[u8]) {
        todo!()
    }
}

impl UartMarker for Noop {}
impl UsbSerialJtagMarker for Noop {}
impl UsbOtgMarker for Noop {}

impl<T: UartMarker> UartMarker for &mut T {}
impl<T: UsbSerialJtagMarker> UsbSerialJtagMarker for &mut T {}
impl<T: UsbOtgMarker> UsbOtgMarker for &mut T {}
