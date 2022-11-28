use core::{cmp::min, mem::size_of, slice};

use md5::{Digest, Md5};
use slip::*;

use crate::{
    commands::{CommandCode::*, Error::*, *},
    miniz_types::*,
    targets::{EspCommon, FLASH_BLOCK_SIZE, FLASH_SECTOR_MASK},
};

const DATA_CMD_SIZE: usize = size_of::<DataCommand>();
const CMD_BASE_SIZE: usize = size_of::<CommandBase>();

const FLASH_SECTOR_SIZE: u32 = 4096;
const MAX_WRITE_BLOCK: u32 = 0x4000;

pub trait InputIO {
    fn recv(&mut self) -> u8;
    fn send(&mut self, data: &[u8]);
}

pub struct Stub<'a> {
    io: &'a mut (dyn InputIO + 'a),
    end_addr: u32,
    write_addr: u32,
    erase_addr: u32,
    remaining: u32,
    remaining_compressed: usize,
    decompressor: tinfl_decompressor,
    last_error: Option<Error>,
    in_flash_mode: bool,
    #[cfg(feature = "esp32c3")]
    target: crate::targets::Esp32c3,
    #[cfg(feature = "esp32")]
    target: crate::targets::Esp32,
    #[cfg(feature = "esp32s3")]
    target: crate::targets::Esp32s3,
}

fn slice_to_struct<T: Sized + Copy>(slice: &[u8]) -> Result<T, Error> {
    if slice.len() < size_of::<T>() {
        return Err(Error::BadDataLen);
    }
    // SAFETY alignment and size have already been checked
    unsafe { Ok(*(slice.as_ptr() as *const T)) }
}

pub unsafe fn to_slice_u8<T: Sized>(p: &T) -> &[u8] {
    slice::from_raw_parts((p as *const T) as *const u8, size_of::<T>())
}

fn u32_from_slice(slice: &[u8], index: usize) -> u32 {
    u32::from_le_bytes(slice[index..index + 4].try_into().unwrap())
}

impl<'a> Stub<'a> {
    pub fn new(input_io: &'a mut dyn InputIO) -> Self {
        let stub = Stub {
            io: input_io,
            write_addr: 0,
            end_addr: 0,
            erase_addr: 0,
            remaining: 0,
            remaining_compressed: 0,
            decompressor: Default::default(),
            last_error: None,
            in_flash_mode: false,
            target: Default::default(),
        };

        stub.target.init();
        stub
    }

    fn send_response(&mut self, resp: &Response) {
        let resp_slice = unsafe { to_slice_u8(resp) };
        write_delimiter(self.io);
        write_raw(self.io, &resp_slice[..RESPONSE_SIZE]);
        write_raw(self.io, resp.data);
        write_delimiter(self.io);
    }

    fn send_response_with_data(&mut self, resp: &Response, data: &[u8]) {
        let resp_slice = unsafe { to_slice_u8(resp) };
        write_delimiter(self.io);
        write_raw(self.io, &resp_slice[..RESPONSE_SIZE - 2]);
        write_raw(self.io, data);
        write_raw(self.io, &resp_slice[RESPONSE_SIZE - 2..RESPONSE_SIZE]);
        write_delimiter(self.io);
    }

    fn send_md5_response(&mut self, resp: &Response, md5: &[u8]) {
        self.send_response_with_data(resp, md5)
    }

    fn send_security_info_response(&mut self, resp: &Response, info: &[u8]) {
        self.send_response_with_data(resp, info)
    }

    pub fn send_greeting(&mut self) {
        let greeting = ['O' as u8, 'H' as u8, 'A' as u8, 'I' as u8];
        write_packet(self.io, &greeting);
    }

    fn calculate_md5(&mut self, mut address: u32, mut size: u32) -> Result<[u8; 16], Error> {
        let mut buffer: [u8; FLASH_SECTOR_SIZE as usize] = [0; FLASH_SECTOR_SIZE as usize];
        let mut hasher = Md5::new();

        while size > 0 {
            let to_read = min(size, FLASH_SECTOR_SIZE);
            self.target.spi_flash_read(address, &mut buffer)?;
            hasher.update(&buffer[0..to_read as usize]);
            size -= to_read;
            address += to_read;
        }

        let result: [u8; 16] = hasher.finalize().into();
        Ok(result)
    }

    fn process_begin(&mut self, cmd: &BeginCommand) -> Result<(), Error> {
        // Align erase addreess to sector boundady.
        self.erase_addr = cmd.offset & FLASH_SECTOR_MASK;
        self.write_addr = cmd.offset;
        self.end_addr = cmd.offset + cmd.total_size;
        self.remaining_compressed = (cmd.packt_count * cmd.packet_size) as usize;
        self.remaining = cmd.total_size;
        self.decompressor.state = 0;
        self.in_flash_mode = true;

        match cmd.base.code {
            FlashBegin | FlashDeflBegin => {
                if cmd.packet_size > MAX_WRITE_BLOCK {
                    return Err(BadBlocksize);
                }
                // Todo: check for 16MB flash only
                self.target.unlock_flash()?;
            }
            _ => (),
        }

        Ok(())
    }

    fn process_flash_end(
        &mut self,
        cmd: &EndFlashCommand,
        response: &Response,
    ) -> Result<(), Error> {
        if !self.in_flash_mode {
            return Err(NotInFlashMode);
        }
        
        if self.remaining > 0 {
            return Err(NotEnoughData);
        }

        self.in_flash_mode = false;

        if cmd.run_user_code == 1 {
            self.send_response(&response);
            self.target.delay_us(10_000);
            self.target.soft_reset();
        }

        Ok(())
    }

    fn process_mem_end(&mut self, cmd: &MemEndCommand, response: &Response) -> Result<(), Error> {
        if self.remaining != 0 {
            return Err(NotEnoughData);
        }
        
        if cmd.stay_in_stub == 0 {
            self.send_response(&response);
            self.target.delay_us(10_000);
            (cmd.entrypoint)();
        }
        
        Ok(())
    }

    fn write_ram(&mut self, data: &[u8]) -> Result<(), Error> {
        let data_len = data.len() as u32;

        if data_len > self.remaining {
            return Err(TooMuchData);
        } 
        
        if data_len % 4 != 0 {
            return Err(BadDataLen);
        }

        let (_, data_u32, _) = unsafe { data.align_to::<u32>() };

        for word in data_u32 {
            let memory = self.write_addr as *mut u32;
            unsafe { *memory = *word };
            self.write_addr += 4;
            self.remaining -= 4;
        }

        Ok(())
    }

    fn flash(&mut self, encrypted: bool, data: &[u8]) -> Result<(), Error> {
        let mut address = self.write_addr;
        let mut remaining = min(self.remaining, data.len() as u32);
        let mut written = 0;

        // Erase flash
        while self.erase_addr < self.write_addr + remaining {
            if self.end_addr >= self.erase_addr + FLASH_BLOCK_SIZE
                && self.erase_addr % FLASH_BLOCK_SIZE == 0
            {
                self.target.flash_erase_block(self.erase_addr)?;
                self.erase_addr += FLASH_BLOCK_SIZE;
            } else {
                self.target.flash_erase_sector(self.erase_addr)?;
                self.erase_addr += FLASH_SECTOR_SIZE;
            }
        }

        // Write flash
        while remaining > 0 {
            let to_write = min(FLASH_SECTOR_SIZE, remaining);
            let data_ptr = data[written..].as_ptr();
            if encrypted {
                self.last_error = self
                    .target
                    .write_encrypted(address, data_ptr, to_write)
                    .err();
            } else {
                self.last_error = self
                    .target
                    .spiflash_write(address, data_ptr, to_write)
                    .err();
            }
            remaining -= to_write;
            written += to_write as usize;
            address += to_write;
        }

        self.write_addr += written as u32;
        self.remaining -= min(self.remaining, written as u32);

        Ok(())
    }

    fn flash_data(&mut self, data: &[u8]) -> Result<(), Error> {
        Ok(self.flash(false, data)?) // NOT SURE
    }

    fn flash_defl_data(&mut self, data: &[u8]) -> Result<(), Error> {
        use crate::miniz_types::TinflStatus::*;

        const OUT_BUFFER_SIZE: usize = 0x8000; // 32768;
        static mut DECOMPRESS_BUF: [u8; OUT_BUFFER_SIZE] = [0; OUT_BUFFER_SIZE];
        static mut DECOMPRESS_INDEX: usize = 0;

        let mut out_index = unsafe { DECOMPRESS_INDEX };
        let out_buf = unsafe { &mut DECOMPRESS_BUF };
        let mut in_index = 0;
        let mut length = data.len();
        let mut status = NeedsMoreInput;
        let mut flags = TINFL_FLAG_PARSE_ZLIB_HEADER;

        while length > 0 && self.remaining > 0 && status != Done {
            let mut in_bytes = length;
            let mut out_bytes = out_buf.len() - out_index;
            let next_out: *mut u8 = out_buf[out_index..].as_mut_ptr();
            
            if self.remaining_compressed > length {
                flags |= TINFL_FLAG_HAS_MORE_INPUT;
            }

            status = self.target.decompress(
                &mut self.decompressor,
                data[in_index..].as_ptr(),
                &mut in_bytes,
                out_buf.as_mut_ptr(),
                next_out,
                &mut out_bytes,
                flags,
            );

            self.remaining_compressed -= in_bytes;
            length -= in_bytes;
            in_index += in_bytes;
            out_index += out_bytes;

            if status == Done || out_index == OUT_BUFFER_SIZE {
                self.flash_data(&out_buf[..out_index])?;
                out_index = 0;
            }
        }

        unsafe { DECOMPRESS_INDEX = out_index };

        // error won't get sent back until next block is sent
        if status < Done {
            self.last_error = Some(Inflate);
        } else if status == Done && self.remaining > 0 {
            self.last_error = Some(NotEnoughData);
        } else if status != Done && self.remaining == 0 {
            self.last_error = Some(TooMuchData);
        }

        Ok(())
    }

    fn flash_encrypt_data(&mut self, data: &[u8]) -> Result<(), Error> {
        self.target.write_encrypted_enable();
        self.flash(true, data)?;
        self.target.write_encrypted_disable();

        Ok(())
    }

    fn process_data(
        &mut self,
        cmd: &DataCommand,
        data: &[u8],
        response: &Response,
    ) -> Result<(), Error> {
        if !self.in_flash_mode {
            return Err(NotInFlashMode);
        }
        
        let checksum: u8 = data.iter().fold(0xEF, |acc, x| acc ^ x);
        
        if cmd.size != data.len() as u32 {
            return Err(BadDataLen);
        }
        
        if cmd.base.checksum != checksum as u32 {
            return Err(BadDataChecksum);
        }
        
        self.send_response(&response);

        match cmd.base.code {
            FlashEncryptedData => self.flash_encrypt_data(data),
            FlashDeflData => self.flash_defl_data(data),
            FlashData => self.flash_data(data),
            MemData => self.write_ram(data),
            _ => Ok(()),
        }
    }

    fn process_read_flash(&mut self, params: &ReadFlashParams) -> Result<(), Error> {
        const BUF_SIZE: usize = FLASH_SECTOR_SIZE as usize;
        let mut buffer: [u8; BUF_SIZE] = [0; BUF_SIZE];
        let mut address = params.address;
        let mut remaining = params.total_size;
        let mut acked = 0;
        let mut ack_buf: [u8; 4] = [0; 4];
        let mut hasher = Md5::new();
        let mut sent = 0;
        let max_inflight_bytes = params.max_inflight * params.packet_size;

        while acked < params.total_size {
            while remaining > 0 && sent < (acked + max_inflight_bytes) {
                let len = min(params.packet_size, remaining);
                self.target
                    .spi_flash_read(address, &mut buffer[..len as usize])?;
                write_packet(self.io, &buffer[..len as usize]);
                hasher.update(&buffer[0..len as usize]);
                remaining -= len;
                address += len;
                sent += len;
            }
            let resp = read_packet(self.io, &mut ack_buf);
            acked = u32_from_slice(resp, 0);
        }

        let md5: [u8; 16] = hasher.finalize().into();
        write_packet(self.io, &md5);
        Ok(())
    }

    #[allow(unreachable_patterns)]
    fn process_cmd(
        &mut self,
        payload: &[u8],
        code: CommandCode,
        response: &mut Response,
    ) -> Result<bool, Error> {
        let mut response_sent = false;

        crate::dprintln!("process command: {:?}", code);

        match code {
            Sync => {
                for _ in 1..=7 {
                    self.send_response(&response);
                }
            }
            ReadReg => {
                let address = u32_from_slice(payload, CMD_BASE_SIZE);
                response.value(self.target.read_register(address));
            }
            WriteReg => {
                let reg: WriteRegCommand = slice_to_struct(payload)?;
                self.target.write_register(reg.address, reg.value);
            }
            FlashBegin | MemBegin | FlashDeflBegin => {
                let cmd: BeginCommand = slice_to_struct(payload)?;
                self.process_begin(&cmd)? //here crashed the S3 chip
            }
            FlashData | FlashDeflData | FlashEncryptedData | MemData => {
                let cmd: DataCommand = slice_to_struct(&payload)?;
                let data = &payload[DATA_CMD_SIZE..];
                self.process_data(&cmd, data, &response)?;
                response_sent = true;
            }
            FlashEnd | FlashDeflEnd => {
                let cmd: EndFlashCommand = slice_to_struct(payload)?;
                self.process_flash_end(&cmd, &response)?;
            }
            MemEnd => {
                let cmd: MemEndCommand = slice_to_struct(payload)?;
                self.process_mem_end(&cmd, &response)?;
            }
            SpiFlashMd5 => {
                let cmd: SpiFlashMd5Command = slice_to_struct(payload)?;
                let md5 = self.calculate_md5(cmd.address, cmd.size)?;
                self.send_md5_response(&response, &md5);
                response_sent = true;
            }
            SpiSetParams => {
                let cmd: SpiSetParamsCommand = slice_to_struct(payload)?;
                self.target.spi_set_params(&cmd.params)?
            }
            SpiAttach => {
                let param = u32_from_slice(payload, CMD_BASE_SIZE);
                self.target.spi_attach(param);
            }
            ChangeBaudrate => {
                let baud: ChangeBaudrateCommand = slice_to_struct(payload)?;
                self.send_response(&response);
                self.target.delay_us(10_000); // Wait for response to be transfered
                self.target.change_baudrate(baud.old, baud.new);
                self.send_greeting();
                response_sent = true;
            }
            EraseFlash => self.target.erase_flash()?,
            EraseRegion => {
                let reg: EraseRegionCommand = slice_to_struct(payload)?;
                self.target.erase_region(reg.address, reg.size)?;
            }
            ReadFlash => {
                self.send_response(&response);
                let cmd: ReadFlashCommand = slice_to_struct(payload)?;
                self.process_read_flash(&cmd.params)?;
                response_sent = true;
            }
            GetSecurityInfo => {
                let info = self.target.get_security_info()?;
                self.send_security_info_response(&response, &info);
                response_sent = true;
            }
            RunUserCode => {
                self.target.soft_reset(); // ESP8266 Only
            }
            _ => {
                return Err(InvalidCommand);
            }
        };

        Ok(response_sent)
    }

    pub fn process_command(&mut self, payload: &[u8]) {
        let command: CommandBase = slice_to_struct(&payload).unwrap();
        let mut response = Response::new(command.code);

        match self.process_cmd(payload, command.code, &mut response) {
            Ok(response_sent) => match response_sent {
                true => return,
                false => (),
            },
            Err(err) => response.error(err),
        }

        self.send_response(&response);
    }

    pub fn read_command<'c, 'd>(&'c mut self, buffer: &'d mut [u8]) -> &'d [u8] {
        read_packet(self.io, buffer)
    }
}

mod slip {
    use super::*;

    pub fn read_packet<'c, 'd>(io: &'c mut dyn InputIO, packet: &'d mut [u8]) -> &'d [u8] {
        while io.recv() != 0xC0 {}

        // Replase: 0xDB 0xDC -> 0xC0 and 0xDB 0xDD -> 0xDB
        let mut i = 0;
        loop {
            match io.recv() {
                0xDB => match io.recv() {
                    0xDC => packet[i] = 0xC0,
                    0xDD => packet[i] = 0xDB,
                    _ => continue, // Framing error, continue processing
                },
                0xC0 => break,
                other => packet[i] = other,
            };
            i += 1;
        }

        &packet[..i]
    }

    pub fn write_raw(io: &mut dyn InputIO, data: &[u8]) {
        for byte in data {
            match byte {
                0xC0 => io.send(&[0xDB, 0xDC]),
                0xDB => io.send(&[0xDB, 0xDD]),
                other => io.send(&[*other]),
            }
        }
    }

    pub fn write_packet(io: &mut dyn InputIO, data: &[u8]) {
        write_delimiter(io);
        write_raw(io, data);
        write_delimiter(io);
    }

    pub fn write_delimiter(io: &mut dyn InputIO) {
        io.send(&[0xC0]);
    }
}
