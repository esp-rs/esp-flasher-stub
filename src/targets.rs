use crate::miniz_types::*;

#[allow(unused)]
extern "C" {
    fn esp_rom_spiflash_erase_chip() -> i32;
    fn esp_rom_spiflash_erase_block(block_number: u32) -> i32;
    fn esp_rom_spiflash_erase_sector(sector_number: u32) -> i32;
    /// address (4 byte alignment), data, length
    fn esp_rom_spiflash_write(dest_addr: u32, data: *const u8, len: u32) -> i32;
    /// address (4 byte alignment), data, length
    fn esp_rom_spiflash_read(src_addr: u32, data: *const u8, len: u32) -> i32;
    fn esp_rom_spiflash_unlock() -> i32;
    fn esp_rom_spiflash_attach(config: u32, legacy: bool);
    fn esp_rom_spiflash_config_param(
        device_id: u32,
        chip_size: u32,
        block_size: u32,
        sector_size: u32,
        page_size: u32,
        status_mask: u32,
    ) -> u32;
    fn uart_tx_one_char(byte: u8);
    fn uart_div_modify(uart_number: u32, baud_div: u32);
    fn ets_efuse_get_spiconfig() -> u32;
    fn software_reset();
    fn ets_delay_us(timeout: u32);
    fn GetSecurityInfoProc(pMsg: u8, pnErr: u8, data: *const u8) -> u32;
    fn esp_rom_spiflash_write_encrypted_enable();
    fn esp_rom_spiflash_write_encrypted_disable();
    fn esp_rom_spiflash_write_encrypted(dest_addr: u32, data: *const u8, len: u32) -> i32;
}

#[cfg_attr(test, mockall::automock)]
pub mod esp32c3 {
    use core::ptr::{read_volatile, write_volatile};

    use super::*;
    use crate::commands::{Error::*, *};

    const SPI_BASE_REG: u32 = 0x60002000;
    const SPI_CMD_REG: u32 = SPI_BASE_REG + 0x00;
    const SPI_ADDR_REG: u32 = SPI_BASE_REG + 0x04;
    const SPI_RD_STATUS_REG: u32 = SPI_BASE_REG + 0x2C;
    const SPI_EXT2_REG: u32 = SPI_BASE_REG + 0x54;

    const SPI0_BASE_REG: u32 = 0x60003000;
    const SPI0_EXT2_REG: u32 = SPI0_BASE_REG + 0x54;

    const SPI_ST: u32 = 0x7;
    const SPI_FLASH_RDSR: u32 = 1 << 27;
    const STATUS_WIP_BIT: u32 = 1 << 0;
    const SPI_FLASH_WREN: u32 = 1 << 30;
    const SPI_FLASH_SE: u32 = 1 << 24;
    const SPI_FLASH_BE: u32 = 1 << 23;

    const UART_BASE_REG: u32 = 0x60000000;
    const UART0_CLKDIV_REG: u32 = UART_BASE_REG + 0x14;
    const UART_CLKDIV_M: u32 = 0x000FFFFF;
    const UART_CLKDIV_FRAG_S: u32 = 20;
    const UART_CLKDIV_FRAG_V: u32 = 0xF;
    pub const FLASH_SECTOR_SIZE: u32 = 4096;
    pub const FLASH_BLOCK_SIZE: u32 = 65536;
    pub const FLASH_SECTOR_MASK: u32 = 0xFFFFF000;
    pub const MAX_WRITE_BLOCK: usize = 0x4000;

    const GPIO_BASE_REG: u32 = 0x60004000;
    const GPIO_STRAP_REG: u32 = GPIO_BASE_REG + 0x38;

    const FLASH_MAX_SIZE: u32 = 16 * 1024 * 1024;
    const FLASH_PAGE_SIZE: u32 = 256;
    const FLASH_STATUS_MASK: u32 = 0xFFFF;

    const SECURITY_INFO_BYTES: usize = 20;

    fn get_uart_div(current_baud: u32, new_baud: u32) -> u32 {
        let clock_div_reg = read_register(UART0_CLKDIV_REG);
        let uart_div = clock_div_reg & UART_CLKDIV_M;
        let fraction = (clock_div_reg >> UART_CLKDIV_FRAG_S) & UART_CLKDIV_FRAG_V;
        let uart_div = (uart_div << 4) + fraction;
        (uart_div * current_baud) / new_baud
    }

    pub fn read_register(address: u32) -> u32 {
        unsafe { read_volatile(address as *const u32) }
    }

    pub fn write_register(address: u32, value: u32) {
        unsafe { write_volatile(address as *mut _, value) }
    }

    pub fn spiflash_write(dest_addr: u32, data: *const u8, len: u32) -> Result<(), Error> {
        match unsafe { esp_rom_spiflash_write(dest_addr, data, len) } {
            0 => Ok(()),
            _ => Err(FailedSpiOp),
        }
    }

    pub fn spi_set_params(params: &SpiParams) -> Result<(), Error> {
        let result = unsafe {
            esp_rom_spiflash_config_param(
                params.id,
                params.total_size,
                params.block_size,
                params.sector_size,
                params.page_size,
                params.status_mask,
            )
        };

        if result == 0 {
            Ok(())
        } else {
            Err(FailedSpiOp)
        }
    }

    pub fn spi_attach(param: u32) {
        unsafe { esp_rom_spiflash_attach(param, false) };
    }

    pub fn change_baudrate(old: u32, new: u32) {
        unsafe { uart_div_modify(0, get_uart_div(old, new)) };
    }

    pub fn erase_flash() -> Result<(), Error> {
        // Returns 1 or 2 in case of failure
        match unsafe { esp_rom_spiflash_erase_chip() } {
            0 => Ok(()),
            _ => Err(FailedSpiOp),
        }
    }

    fn erase(address: u32, block: bool) {
        spiflash_wait_for_ready();
        spi_write_enable();
        wait_for_ready();

        let command = if block { SPI_FLASH_BE } else { SPI_FLASH_SE };
        write_register(SPI_ADDR_REG, address);
        write_register(SPI_CMD_REG, command);
        while read_register(SPI_CMD_REG) != 0 {}

        spiflash_wait_for_ready();
    }

    fn wait_for_ready() {
        while (read_register(SPI_EXT2_REG) & SPI_ST) != 0 {}
        while (read_register(SPI0_EXT2_REG) & SPI_ST) != 0 {} // ESP32_OR_LATER
    }

    fn spiflash_wait_for_ready() {
        wait_for_ready();

        write_register(SPI_RD_STATUS_REG, 0);
        write_register(SPI_CMD_REG, SPI_FLASH_RDSR);
        while read_register(SPI_CMD_REG) != 0 {}
        while (read_register(SPI_RD_STATUS_REG) & STATUS_WIP_BIT) != 0 {}
    }

    fn spi_write_enable() {
        write_register(SPI_CMD_REG, SPI_FLASH_WREN);
        while read_register(SPI_CMD_REG) != 0 {}
    }

    pub fn flash_erase_block(address: u32) {
        // unsafe{ esp_rom_spiflash_erase_block(address / FLASH_BLOCK_SIZE) };
        erase(address, true);
    }

    pub fn flash_erase_sector(address: u32) {
        erase(address, false);
    }

    pub fn erase_region(address: u32, size: u32) -> Result<(), Error> {
        if address % FLASH_SECTOR_SIZE != 0 {
            return Err(Err0x32);
        } else if size % FLASH_SECTOR_SIZE != 0 {
            return Err(Err0x33);
        } else if unsafe { esp_rom_spiflash_unlock() } != 0 {
            return Err(Err0x34);
        }

        let sector_start = address / FLASH_SECTOR_SIZE;
        let sector_end = sector_start + (size / FLASH_SECTOR_SIZE);

        for sector in sector_start..sector_end {
            if unsafe { esp_rom_spiflash_erase_sector(sector) } != 0 {
                return Err(Err0x35);
            }
        }

        Ok(())
    }

    pub fn spi_flash_read(address: u32, data: &mut [u8]) -> Result<(), Error> {
        let data_ptr = data.as_mut_ptr();
        let data_len = data.len() as u32;

        match unsafe { esp_rom_spiflash_read(address, data_ptr, data_len) } {
            0 => Ok(()),
            _ => Err(Err0x63),
        }
    }

    pub fn unlock_flash() -> Result<(), Error> {
        if unsafe { esp_rom_spiflash_unlock() } != 0 {
            Err(FailedSpiUnlock)
        } else {
            Ok(())
        }
    }

    // ESP32S2_OR_LATER && !ESP32H2BETA2
    pub fn get_security_info() -> Result<[u8; SECURITY_INFO_BYTES], Error> {
        let mut buf: [u8; SECURITY_INFO_BYTES] = [0; SECURITY_INFO_BYTES];

        match unsafe { GetSecurityInfoProc(0, 0, buf.as_mut_ptr()) } {
            0 => Ok(buf),
            _ => Err(InvalidCommand), // Todo check ROM code for err val
        }
    }

    pub fn init() {
        let mut spiconfig = unsafe { ets_efuse_get_spiconfig() };

        let strapping = read_register(GPIO_STRAP_REG);

        if spiconfig == 0 && (strapping & 0x1c) == 0x08 {
            spiconfig = 1; // HSPI flash mode
        }

        spi_attach(spiconfig);

        let deafault_params = SpiParams {
            id: 0,
            total_size: FLASH_MAX_SIZE,
            block_size: FLASH_BLOCK_SIZE,
            sector_size: FLASH_SECTOR_SIZE,
            page_size: FLASH_PAGE_SIZE,
            status_mask: FLASH_STATUS_MASK,
        };

        let _ = spi_set_params(&deafault_params);
    }

    pub fn soft_reset() {
        unsafe { software_reset() };
    }

    pub fn delay_us(micro_seconds: u32) {
        unsafe { ets_delay_us(micro_seconds) };
    }

    pub fn write_encrypted_enable() {
        unsafe {
            esp_rom_spiflash_write_encrypted_enable();
        }
    }
    pub fn write_encrypted_disable() {
        unsafe {
            esp_rom_spiflash_write_encrypted_disable();
        }
    }

    pub fn write_encrypted(addr: u32, data: *const u8, len: u32) -> Result<(), Error> {
        match unsafe { esp_rom_spiflash_write_encrypted(addr, data, len) } {
            0 => Ok(()),
            _ => Err(FailedSpiOp),
        }
    }

    pub fn decompress(
        r: *mut tinfl_decompressor,
        in_buf: *const u8,
        in_buf_size: *mut usize,
        out_buf_start: *mut u8,
        out_buf_next: *mut u8,
        out_buf_size: *mut usize,
        flags: u32,
    ) -> TinflStatus {
        unsafe {
            tinfl_decompress(
                r,
                in_buf,
                in_buf_size,
                out_buf_start,
                out_buf_next,
                out_buf_size,
                flags,
            )
        }
    }
}
