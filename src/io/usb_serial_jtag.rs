use super::{UsbSerialJtagMarker, RX_QUEUE};
use crate::{
    hal::{
        peripherals::USB_DEVICE,
        prelude::handler,
        usb_serial_jtag::{Instance, UsbSerialJtag},
        Blocking,
    },
    protocol::InputIO,
};

impl InputIO for UsbSerialJtag<'_, Blocking> {
    fn recv(&mut self) -> u8 {
        unsafe {
            while critical_section::with(|_| RX_QUEUE.is_empty()) {}
            critical_section::with(|_| RX_QUEUE.pop_front().unwrap())
        }
    }

    fn send(&mut self, bytes: &[u8]) {
        self.write_bytes(bytes).unwrap()
    }
}

impl UsbSerialJtagMarker for UsbSerialJtag<'_, Blocking> {}

#[handler]
pub fn usb_device_handler() {
    let reg_block = USB_DEVICE::register_block();

    while reg_block
        .ep1_conf()
        .read()
        .serial_out_ep_data_avail()
        .bit_is_set()
    {
        unsafe {
            RX_QUEUE
                .push_back(reg_block.ep1().read().rdwr_byte().bits())
                .unwrap()
        };
    }

    reg_block
        .int_clr()
        .write(|w| w.serial_out_recv_pkt().clear_bit_by_one());
}
