use core::ptr::{read_volatile, write_volatile};

use crate::{
    commands::{Error::*, *},
    miniz_types::*,
};

#[repr(C, packed(1))]
struct RomSpiFlashChip {
    device_id: u32,
    chip_size: u32,
    block_size: u32,
    sector_size: u32,
    page_size: u32,
    status_mask: u32,
}

// ROM SPIFLASH functions can be found here:
// https://github.com/espressif/esp-idf/tree/master/components/esp_rom or
// https://github.com/espressif/esptool/tree/master/flasher_stub/ld
#[allow(unused)]
extern "C" {
    fn esp_rom_spiflash_erase_chip() -> i32;
    fn esp_rom_spiflash_erase_block(block_number: u32) -> i32;
    fn esp_rom_spiflash_erase_sector(sector_number: u32) -> i32;
    fn esp_rom_spiflash_erase_area(start_addr: u32, len: u32) -> i32;
    fn esp_rom_spiflash_write(dest_addr: u32, data: *const u8, len: u32) -> i32;
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
    fn esp_rom_spiflash_wait_idle() -> i32;
    fn uart_tx_one_char(byte: u8);
    fn uart_div_modify(uart_number: u32, baud_div: u32);
    fn ets_efuse_get_spiconfig() -> u32;
    fn software_reset();
    fn ets_delay_us(timeout: u32);
    fn get_security_info_proc(pMsg: u8, pnErr: u8, data: *const u8) -> u32;
    fn esp_rom_spiflash_write_encrypted_enable();
    fn esp_rom_spiflash_write_encrypted_disable();
    fn spi_write_status(chip: *const RomSpiFlashChip, status: u32) -> u32;
    fn esp_rom_spiflash_write_encrypted(dest_addr: u32, data: *const u8, len: u32) -> i32;
    fn spi_read_status_high(status: *const u32) -> u32;
}

const SECURITY_INFO_BYTES: usize = 20;

pub const FLASH_SECTOR_SIZE: u32 = 4096;
pub const FLASH_BLOCK_SIZE: u32 = 65536;
pub const FLASH_SECTOR_MASK: u32 = 0xFFFFF000;
pub const MAX_WRITE_BLOCK: usize = 0x4000;

pub trait EspCommon {
    const SPI_BASE_REG: u32 = 0x60002000;
    const SPI_RD_STATUS_REG: u32 = Self::SPI_BASE_REG + 0x2C;
    const SPI_EXT2_REG: u32 = Self::SPI_BASE_REG + 0x54;
    const SPI0_BASE_REG: u32 = 0x60003000;
    const SPI0_EXT2_REG: u32 = Self::SPI0_BASE_REG + 0x54;
    const UART_BASE_REG: u32 = 0x60000000;
    const GPIO_BASE_REG: u32 = 0x60004000;

    const SPI_CMD_REG: u32 = Self::SPI_BASE_REG + 0x00;
    const SPI_ADDR_REG: u32 = Self::SPI_BASE_REG + 0x04;
    const SPI_CTRL_REG: u32 = Self::SPI_BASE_REG + 0x08;

    const SPI_ST: u32 = 0x7;
    const SPI_FLASH_RDSR: u32 = 1 << 27;
    const STATUS_WIP_BIT: u32 = 1 << 0;
    const SPI_FLASH_WREN: u32 = 1 << 30;
    const SPI_FLASH_SE: u32 = 1 << 24;
    const SPI_FLASH_BE: u32 = 1 << 23;

    const UART0_CLKDIV_REG: u32 = Self::UART_BASE_REG + 0x14;
    const UART_CLKDIV_M: u32 = 0x000FFFFF;
    const UART_CLKDIV_FRAG_S: u32 = 20;
    const UART_CLKDIV_FRAG_V: u32 = 0xF;

    const GPIO_STRAP_REG: u32 = Self::GPIO_BASE_REG + 0x38;

    const FLASH_MAX_SIZE: u32 = 16 * 1024 * 1024;
    const FLASH_PAGE_SIZE: u32 = 256;
    const FLASH_STATUS_MASK: u32 = 0xFFFF;

    const SECURITY_INFO_BYTES: usize = 20;

    fn get_uart_div(&self, current_baud: u32, new_baud: u32) -> u32 {
        let clock_div_reg = self.read_register(Self::UART0_CLKDIV_REG);
        let uart_div = clock_div_reg & Self::UART_CLKDIV_M;
        let fraction = (clock_div_reg >> Self::UART_CLKDIV_FRAG_S) & Self::UART_CLKDIV_FRAG_V;
        let uart_div = (uart_div << 4) + fraction;
        (uart_div * current_baud) / new_baud
    }

    fn read_register(&self, address: u32) -> u32 {
        unsafe { read_volatile(address as *const u32) }
    }

    fn write_register(&self, address: u32, value: u32) {
        unsafe { write_volatile(address as *mut _, value) }
    }

    fn set_register_mask(&self, address: u32, mask: u32) {
        self.write_register(address, self.read_register(address) | mask);
    }

    fn spiflash_write(&self, dest_addr: u32, data: *const u8, len: u32) -> Result<(), Error> {
        match unsafe { esp_rom_spiflash_write(dest_addr, data, len) } {
            0 => Ok(()),
            _ => Err(FailedSpiOp),
        }
    }

    fn spi_set_params(&self, params: &SpiParams) -> Result<(), Error> {
        match unsafe {
            esp_rom_spiflash_config_param(
                params.id,
                params.total_size,
                params.block_size,
                params.sector_size,
                params.page_size,
                params.status_mask,
            )
        } {
            0 => Ok(()),
            _ => Err(FailedSpiOp),
        }
    }

    fn spi_attach(&self, param: u32) {
        unsafe { esp_rom_spiflash_attach(param, false) };
    }

    fn change_baudrate(&self, old: u32, new: u32) {
        unsafe { uart_div_modify(0, self.get_uart_div(old, new)) };
    }

    fn erase_flash(&self) -> Result<(), Error> {
        // Returns 1 or 2 in case of failure
        match unsafe { esp_rom_spiflash_erase_chip() } {
            0 => Ok(()),
            _ => Err(FailedSpiOp),
        }
    }

    fn wait_for_ready(&self) {
        while (self.read_register(Self::SPI_EXT2_REG) & Self::SPI_ST) != 0 {}
        while (self.read_register(Self::SPI0_EXT2_REG) & Self::SPI_ST) != 0 {} // ESP32_OR_LATER
    }

    fn spiflash_wait_for_ready(&self) {
        while unsafe { esp_rom_spiflash_wait_idle() } != 0 {} 
    }

    fn spi_write_enable(&self) {
        self.write_register(Self::SPI_CMD_REG, Self::SPI_FLASH_WREN);
        while self.read_register(Self::SPI_CMD_REG) != 0 {}
    }

    fn flash_erase_block(&self, address: u32) -> Result<(), Error> {
        match unsafe { esp_rom_spiflash_erase_block(address / FLASH_BLOCK_SIZE) } {
            0 => Ok(()),
            _ => Err(EraseErr),
        }
    }

    fn flash_erase_sector(&self, address: u32) -> Result<(), Error> {
        match unsafe { esp_rom_spiflash_erase_sector(address / FLASH_SECTOR_SIZE) } {
            0 => Ok(()),
            _ => Err(EraseErr),
        }
    }

    fn erase_region(&self, address: u32, size: u32) -> Result<(), Error> {
        match unsafe { esp_rom_spiflash_erase_area(address, size) } {
            0 => Ok(()),
            _ => Err(EraseErr),
        }
    }

    fn spi_flash_read(&self, address: u32, data: &mut [u8]) -> Result<(), Error> {
        let data_ptr = data.as_mut_ptr();
        let data_len = data.len() as u32;

        match unsafe { esp_rom_spiflash_read(address, data_ptr, data_len) } {
            0 => Ok(()),
            _ => Err(Err0x63),
        }
    }

    fn unlock_flash(&self) -> Result<(), Error> {
        match unsafe { esp_rom_spiflash_unlock() } {
            0 => Ok(()),
            _ => Err(FailedSpiUnlock), // TODO: add also timeout error
        }
    }

    fn get_security_info(&self) -> Result<[u8; SECURITY_INFO_BYTES], Error> {
        let mut buf: [u8; SECURITY_INFO_BYTES] = [0; SECURITY_INFO_BYTES];

        match unsafe { get_security_info_proc(0, 0, buf.as_mut_ptr()) } {
            0 => Ok(buf),
            _ => Err(InvalidCommand), // Todo check ROM code for err val
        }
    }

    fn init(&self) {
        let mut spiconfig = unsafe { ets_efuse_get_spiconfig() };

        let strapping = self.read_register(Self::GPIO_STRAP_REG);

        if spiconfig == 0 && (strapping & 0x1c) == 0x08 {
            spiconfig = 1; // HSPI flash mode
        }

        self.spi_attach(spiconfig);

        let deafault_params = SpiParams {
            id: 0,
            total_size: Self::FLASH_MAX_SIZE,
            block_size: FLASH_BLOCK_SIZE,
            sector_size: FLASH_SECTOR_SIZE,
            page_size: Self::FLASH_PAGE_SIZE,
            status_mask: Self::FLASH_STATUS_MASK,
        };

        let _ = self.spi_set_params(&deafault_params);
    }

    fn soft_reset(&self) {
        unsafe { software_reset() };
    }

    fn delay_us(&self, micro_seconds: u32) {
        unsafe { ets_delay_us(micro_seconds) };
    }

    fn write_encrypted_enable(&self) {
        unsafe {
            esp_rom_spiflash_write_encrypted_enable();
        }
    }
    fn write_encrypted_disable(&self) {
        unsafe {
            esp_rom_spiflash_write_encrypted_disable();
        }
    }

    fn write_encrypted(&self, addr: u32, data: *const u8, len: u32) -> Result<(), Error> {
        match unsafe { esp_rom_spiflash_write_encrypted(addr, data, len) } {
            0 => Ok(()),
            _ => Err(FailedSpiOp),
        }
    }

    fn decompress(
        &self,
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
#[derive(Default)]
pub struct Esp32c3;

#[derive(Default)]
pub struct Esp32;

#[derive(Default)]
pub struct Esp32s3;

impl EspCommon for Esp32c3 {}

impl EspCommon for Esp32 {
    const SPI_BASE_REG: u32 = 0x3ff42000;
    const SPI_RD_STATUS_REG: u32 = Self::SPI_BASE_REG + 0x10;
    const SPI_EXT2_REG: u32 = Self::SPI_BASE_REG + 0xF8;
    const SPI0_BASE_REG: u32 = 0x3ff43000;
    const SPI0_EXT2_REG: u32 = Self::SPI0_BASE_REG + 0xF8;
    const UART_BASE_REG: u32 = 0x3ff40000;
    const GPIO_BASE_REG: u32 = 0x3ff44000;

    fn get_security_info(&self) -> Result<[u8; SECURITY_INFO_BYTES], Error> {
        Err(InvalidCommand)
    }

    // the ROM function has been replaced with patched code so we have to "override" it 
    // https://github.com/espressif/esp-idf/blob/master/components/esp_rom/esp32/ld/esp32.rom.spiflash.ld#L23
    fn unlock_flash(&self) -> Result<(), Error> {
        let mut status: u32 = 0;
        const STATUS_QIE_BIT: u32 = 1 << 9;
        const SPI_WRSR_2B: u32 = 1 << 22;
        const FLASHCHIP: *const RomSpiFlashChip = 0x3ffae270 as *const RomSpiFlashChip;

        self.wait_for_ready();

        if (unsafe { spi_read_status_high(&status) } != 0) {
            return Err(FailedSpiUnlock);
        }

        // Clear all bits except QIE, if it is set.
        // (This is different from ROM SPIUnlock, which keeps all bits as-is.)
        status &= STATUS_QIE_BIT;

        self.spi_write_enable();
        self.set_register_mask(Self::SPI_CTRL_REG, SPI_WRSR_2B);

        if (unsafe { spi_write_status(FLASHCHIP, status) } != 0) {
            return Err(FailedSpiUnlock);
        }

        Ok(())
    }
}

impl EspCommon for Esp32s3 {}
