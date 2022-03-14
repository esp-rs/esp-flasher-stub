
#[cfg(test)]
use mockall::automock;

#[allow(unused)]
extern "C" {
    fn esp_rom_spiflash_erase_chip() -> i32;
    fn esp_rom_spiflash_erase_block(block_number: u32) -> i32;
    fn esp_rom_spiflash_erase_sector(sector_number: u32) -> i32;
    /// address (4 byte alignment), data, length
    fn esp_rom_spiflash_write(dest_addr: u32, data: *const u8, len: u32) -> i32;
    /// address (4 byte alignment), data, length
    fn esp_rom_spiflash_read(src_addr: u32, data: *const u32, len: u32) -> i32;
    fn esp_rom_spiflash_unlock() -> i32;
    // fn esp_rom_spiflash_lock(); // can't find in idf defs?
    fn esp_rom_spiflash_attach(config: u32, legacy: bool);

    fn uart_tx_one_char(byte: u8);

    fn ets_efuse_get_spiconfig() -> u32;
}


#[cfg_attr(test, automock)]
pub mod esp32c3 {
    use crate::commands::*;
    use super::*;

    pub fn read_register(address: u32) -> u32 {
        unsafe { *(address as *const u32) }
    }
    
    pub fn write_register(address: u32, value: u32) {
        // let reg_ptr = address as *mut u32;
        unsafe { *(address as *mut u32) = value; }
        todo!();
    }

    pub fn memory_write(_mem_type: CommandCode, address: u32, data: &[u8]) -> Result<(), Error>{
        let err = unsafe { esp_rom_spiflash_write(address, data.as_ptr(), data.len() as u32) };
        
        if err == 0 { Ok(()) } else { Err(Error::FailedSpiOp) } }

    pub fn run_user_code() {
        todo!();
    }

    pub fn spi_set_params(_params: &SpiParams) {
        todo!();
    }

    pub fn spi_attach(_param: u32) {
        todo!();
    }

    pub fn change_baudrate(_old: u32, _new: u32) {
        todo!();
    }

    pub fn erase_flash() -> Result<(), Error> {
        // Can return FailedSpiOp (1, 2)
        todo!();
    }

    pub fn erase_region(_address: u32, _size: u32) -> Result<(), Error> {
        // Can return FailedSpiOp (?)
        todo!();
    }

    pub fn read_flash(_params: &ReadFlashParams) -> Result<(), Error> {
        // Can return FailedSpiOp (?)
        todo!();
    }
}
