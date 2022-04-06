#![allow(dead_code)]

use mockall_double::double;

#[double]
use crate::targets::esp32c3 as target;

#[derive(Debug, PartialEq)]
pub enum ErrorIO {
    Overflow,
    Incomplete,
    InvalidResponse,
    Hardware,
}

pub trait InputIO {
    fn read(&mut self) -> Result<u8, ErrorIO>;
    fn write(&mut self, data: &[u8]) -> Result<(), ErrorIO>;
}


type Buffer = heapless::Vec<u8, 0x5000>;

pub mod stub
{
    use super::*;
    use super::target::*;
    use super::slip::*;
    use core::mem;
    use core::slice;
    use core::cmp::min;
    // use core::mem::MaybeUninit;
    use crate::commands::*;
    use crate::commands::CommandCode::*;
    use md5::{Md5, Digest};
    
    const DATA_CMD_SIZE: usize = mem::size_of::<DataCommand>();
    const CMD_BASE_SIZE: usize = mem::size_of::<CommandBase>();
    
    const FLASH_SECTOR_SIZE: u32 = 4096;
    const MAX_WRITE_BLOCK: u32 = 0x4000;
    pub struct Stub<'a> {
        io: &'a mut (dyn InputIO + 'a),
        end_addr: u32,
        write_addr: u32,
        erase_addr: u32,
    }

    impl From<ErrorIO> for Error {
        fn from(_: ErrorIO) -> Self {
            Error::FailedSpiOp
        }
    }
    
    // todo: get rid of SIZE
    fn slice_to_struct<T: Sized, const SIZE: usize>(slice: &[u8]) -> Result<T, Error>
    {
        if SIZE != slice.len() {
            return Err(Error::BadDataLen);
        }
        let array: &[u8; SIZE] = &slice[0..SIZE].try_into().unwrap();
        unsafe { Ok(mem::transmute_copy::<[u8; SIZE], T>(array)) }
    }

    macro_rules! transmute {
        ($slice:expr, $type:ty) => {
            slice_to_struct::<$type, { mem::size_of::<$type>() }>($slice)
        };
    }

    pub unsafe fn to_slice_u8<T: Sized>(p: &T) -> &[u8] {
        slice::from_raw_parts( (p as *const T) as *const u8, mem::size_of::<T>(), )
    }

    fn u32_from_slice(slice: &[u8], index: usize) -> u32 {
        u32::from_le_bytes(slice[index..index+4].try_into().unwrap())
    }
    

    impl<'a> Stub<'a> {

        pub fn new(input_io: &'a mut dyn InputIO) -> Self {
            Stub { 
                io: input_io,
                write_addr: 0,
                end_addr: 0,
                erase_addr: 0,
            }
        }

        pub fn send_response(&mut self, resp: &Response) -> Result<(), Error> {
            let resp_slice = unsafe{ to_slice_u8(resp) };
            write_delimiter(self.io)?;
            write_packet(self.io, &resp_slice[..RESPONSE_SIZE])?;
            write_packet(self.io, resp.data)?;
            write_delimiter(self.io)?;
            Ok(())
        }

        pub fn send_md5_response(&mut self, resp: &Response, md5: &[u8]) -> Result<(), Error> {
            let resp_slice = unsafe{ to_slice_u8(resp) };
            write_delimiter(self.io)?;
            write_packet(self.io, &resp_slice[..RESPONSE_SIZE-2])?;
            write_packet(self.io, md5)?;
            write_packet(self.io, &resp_slice[RESPONSE_SIZE-2..RESPONSE_SIZE])?;
            write_delimiter(self.io)?;
            Ok(())
        }

        fn process_begin(&mut self, cmd: &BeginCommand) -> Result<(), Error> {
            
            match cmd.base.code {
                FlashBegin => {
                    if cmd.packet_size > MAX_WRITE_BLOCK {
                        return Err(Error::BadBlocksize);
                    }
                    self.write_addr = cmd.offset;
                    self.erase_addr = cmd.offset;
                    self.end_addr = cmd.offset + cmd.total_size;
                    // Todo check max size 16MB
                    return unlock_flash();
                },
                MemBegin => {
                    self.write_addr = cmd.offset;
                    self.end_addr = cmd.offset + cmd.total_size;
                }
                _ => ()
            }
            Ok(())
        }
        
        fn process_data(&mut self, cmd: &DataCommand, data: &[u8]) -> Result<(), Error> {
            let checksum: u8 = data.iter().fold(0xEF, |acc, x| acc ^ x);
            let code = cmd.base.code;
            let data_len = data.len() as u32;

            if cmd.size != data_len {
                return Err(Error::BadDataLen);
            } else if cmd.base.checksum != checksum as u32 {
                return Err(Error::BadDataChecksum);
            } else {
                while self.erase_addr < self.write_addr + data_len {
                    if self.end_addr >= self.erase_addr + FLASH_BLOCK_SIZE && 
                        self.erase_addr % FLASH_BLOCK_SIZE == 0 {
                        flash_erase_block(self.erase_addr);
                        self.erase_addr += FLASH_BLOCK_SIZE;
                    } else {
                        flash_erase_sector(self.erase_addr);
                        self.erase_addr += FLASH_SECTOR_SIZE;
                    }
                }
                memory_write(code, self.write_addr, data)?;                    
                self.write_addr += data_len
            }
            Ok(())
        }

        #[allow(unreachable_patterns)]
        pub fn process_cmd(&mut self, payload: &[u8], 
                            code: CommandCode, 
                            response: &mut Response) -> Result<bool, Error> {
            
            let mut response_sent = false;
    
            match code {
                Sync => (),
                ReadReg => {
                    let address = u32_from_slice(payload, CMD_BASE_SIZE);
                    response.value(read_register(address));
                }
                WriteReg => {
                    let reg = transmute!(payload, WriteRegCommand)?;
                    write_register(reg.address, reg.value);
                }
                FlashBegin | MemBegin | FlashDeflBegin => {
                    let cmd = transmute!(payload, BeginCommand)?;
                    self.process_begin(&cmd)?
                }
                FlashData | FlashDeflData | MemData => {
                    let cmd = transmute!(&payload[..DATA_CMD_SIZE], DataCommand)?;
                    let data = &payload[DATA_CMD_SIZE..];
                    self.process_data(&cmd, data)?
                }
                FlashEnd | MemEnd | FlashDeflEnd => {
                    let cmd = transmute!(payload, EndCommand)?;
                    if cmd.run_user_code == 1 {
                        self.send_response(&response)?;
                        delay_us(10000);
                        soft_reset();
                    }
                }
                SpiFlashMd5 => {
                    let cmd = transmute!(payload, SpiFlashMd5Command)?;
                    let md5 = calculate_md5(cmd.address, cmd.size)?;
                    self.send_md5_response(&response, &md5)?;
                    response_sent = true;
                }
                SpiSetParams => {
                    let cmd = transmute!(payload, SpiSetParamsCommand)?;
                    spi_set_params(&cmd.params)?
                }
                SpiAttach => {
                    let param = u32_from_slice(payload, CMD_BASE_SIZE);
                    spi_attach(param);
                }
                ChangeBaudrate => {
                    let baud = transmute!(payload, ChangeBaudrateCommand)?;
                    self.send_response(&response)?;
                    delay_us(10000);
                    change_baudrate(baud.old, baud.new);
                    response_sent = true;
                }
                EraseFlash => {
                    erase_flash()?
                }
                EraseRegion => {
                    let reg = transmute!(payload, EraseRegionCommand)?;
                    erase_region(reg.address, reg.size)?
                }
                ReadFlash => {
                    let cmd = transmute!(payload, ReadFlashCommand)?;
                    read_flash(&cmd.params)?
                }
                RunUserCode => {
                    todo!();
                }
                _ => {
                    return Err(Error::InvalidCommand);
                }
            };

            Ok(response_sent)
        }

        #[allow(unreachable_patterns)]
        pub fn process_command(&mut self, payload: &[u8]) -> Result<(), Error> {
            
            let command = transmute!(&payload[..CMD_BASE_SIZE], CommandBase)?;
            let mut response = Response::new(command.code);

            match self.process_cmd(payload, command.code, &mut response) {
                Ok(response_sent) =>  match response_sent {
                    true => return Ok(()),
                    false => (),
                }
                Err(err) => response.error(err)
            }

            self.send_response(&response)
        }

        pub fn read_command(&mut self, buffer: &mut Buffer) -> Result<(), Error> {
            read_packet(self.io, buffer)?;
            Ok(())
        }
    }

    pub fn calculate_md5(mut address: u32, mut size: u32) -> Result<[u8; 16], Error> {
        let mut buffer: [u8; FLASH_SECTOR_SIZE as usize] = [0; FLASH_SECTOR_SIZE as usize];
        let mut hasher = Md5::new();

        while size > 0 {
            let to_read = min(size, FLASH_SECTOR_SIZE);
            target::spi_flash_read(address, &mut buffer)?;
            hasher.update(&buffer[0..to_read as usize]);
            size -= to_read;
            address += to_read;
        }

        let result: [u8; 16] = hasher.finalize().into();
        Ok(result)
    }
}

// Check command and its size, cast it 
mod slip {
    use super::*;
    use super::ErrorIO::*;

    impl From<u8> for ErrorIO {
        fn from(_: u8) -> Self {
            ErrorIO::Overflow
        }
    }

    pub fn read_packet(io: &mut dyn InputIO, packet: &mut Buffer) -> Result<(), ErrorIO> {
        packet.clear();
        
        while io.read()? != 0xC0 { }

        // Replase: 0xDB 0xDC -> 0xC0 and 0xDB 0xDD -> 0xDB   
        loop {
            match io.read()? {
                0xDB => match io.read()? {
                    0xDC => packet.push(0xC0)?,
                    0xDD => packet.push(0xDB)?,
                    _ => return Err(InvalidResponse),
                }
                0xC0 => break,
                other => packet.push(other)?,
            };
        }
        Ok(())
    } 

    pub fn write_packet(io: &mut dyn InputIO, packet: &[u8]) -> Result<(), ErrorIO> {
        for byte in packet {
            match byte {
                0xC0  => io.write(&[0xDB, 0xDC])?,
                0xDB  => io.write(&[0xDB, 0xDD])?,
                other => io.write(&[*other])?,
            }
        }
        Ok(())
    }
    
    pub fn write_delimiter(io: &mut dyn InputIO) -> Result<(), ErrorIO> {
        io.write(&[0xC0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::ErrorIO::*;
    use super::stub::*;
    // use super::stub::Error::*;
    use super::slip::{read_packet, write_packet};
    use assert2::{assert, let_assert};
    // use matches::assert_matches;
    use std::collections::VecDeque;
    use std::vec::Vec;
    use crate::commands::*;
    use mockall::predicate;

    struct MockIO {
        data: VecDeque<u8>
    }

    impl MockIO {
        fn from_slice(bytes: &[u8]) -> Self {
            let bytes_vec = Vec::from(bytes);
            MockIO { data: VecDeque::from(bytes_vec) }
        }

        fn new() -> Self {
            MockIO { data: VecDeque::new() }
        }

        fn fill(&mut self, bytes: &[u8]) {
            self.data.clear();
            self.data.extend(bytes);
        }

        fn clear(&mut self) {
            self.data.clear();
        }

        fn written(&mut self) -> &[u8] {
            self.data.make_contiguous()
        }
    }

    impl InputIO for MockIO {
        fn read(&mut self) -> Result<u8, ErrorIO> {
            match self.data.pop_front() {
                Some(top) => Ok(top),
                None => Err(Incomplete)
            }
        }

        fn write(&mut self, bytes: &[u8]) -> Result<(), ErrorIO>
        {
            self.data.extend(bytes);
            Ok(())
        }
    }

    #[test]
    fn test_read_packet() {
        let mut io = MockIO::new();
        let mut buffer: Buffer = heapless::Vec::new();
        
        // Returns Incomplete when packet enclosed by 0xC0 was not found 
        io.fill(&[0xC0, 0xAA, 0x22]);
        assert!( read_packet(&mut io, &mut buffer) == Err(Incomplete));
        
        // Returns Incomplete when no 0xC0 is found
        io.fill(&[0x00, 0xAA, 0x22]);
        assert!( read_packet(&mut io, &mut buffer) == Err(Incomplete));

        // Can find packet by 0xC0
        io.fill(&[0xC0, 0x11, 0x22, 0xC0]);
        assert!( read_packet(&mut io, &mut buffer) == Ok(()));
        assert!( buffer.as_slice() == &[0x11, 0x22] );
        
        // Can find packet by 0xC0
        io.fill(&[0xC0, 0x11, 0x22, 0xC0]);
        assert!( read_packet(&mut io, &mut buffer) == Ok(()));
        assert!( buffer.as_slice() == &[0x11, 0x22] );

        // Can convert 0xDB 0xDC -> 0xC0
        io.fill(&[0xC0, 0x11, 0xDB, 0xDC, 0x22, 0xC0]);
        assert!( read_packet(&mut io, &mut buffer) == Ok(()));
        assert!( buffer.as_slice() == &[0x11, 0xC0, 0x22] );

        // Can convert 0xDB 0xDD -> 0xDB
        io.fill(&[0xC0, 0x11, 0xDB, 0xDD, 0x22, 0xC0]);
        assert!( read_packet(&mut io, &mut buffer) == Ok(()));
        assert!( buffer.as_slice() == &[0x11, 0xDB, 0x22] );

        // Returns InvalidResponse after invalid byte pair
        io.fill(&[0xC0, 0x11, 0xDB, 0x22, 0xDB, 0x33, 0x44, 0xC0]);
        assert!( read_packet(&mut io, &mut buffer) == Err(InvalidResponse));
    }

    #[test]
    fn test_write_packet() {
        let mut io = MockIO::new();

        // 0xC0 is replaced with 0xDB 0xDC
        assert!( write_packet(&mut io, &[1, 0xC0, 3]) == Ok(()));
        assert!( io.written() == &[1, 0xDB, 0xDC, 3] );
        io.clear();

        // 0xDB is replaced with 0xDB 0xDD
        assert!( write_packet(&mut io, &[1, 0xDB, 3]) == Ok(()));
        assert!( io.written() == &[1, 0xDB, 0xDD, 3] );
        io.clear();
    }

    #[test]
    fn test_wait_for_packet() {
        let mut dummy = CommandCode::Sync;
        // Check FlashBegin command
        let mut io = MockIO::from_slice(&[
            0xC0, 
            0,          // direction
            CommandCode::FlashBegin as u8,
            16, 0,      // size
            1, 0, 0, 0, // checksum
            2, 0, 0, 0, // erase_addr
            3, 0, 0, 0, // packt_count
            4, 0, 0, 0, // packet_size
            5, 0, 0, 0, // offset
            0xC0]);
        let mut stub = Stub::new(&mut io);
        let_assert!( Ok(Command::Begin(cmd)) = stub.wait_for_command(&mut dummy) );
        assert!( {cmd.base.direction == 0} );
        assert!( {cmd.base.code == CommandCode::FlashBegin} );
        assert!( {cmd.base.size == 16} );
        assert!( {cmd.base.checksum == 1} );
        assert!( {cmd.total_size == 2} );
        assert!( {cmd.packt_count == 3} );
        assert!( {cmd.packet_size == 4} );
        assert!( {cmd.offset == 5} );

        // Check FlashData command
        let mut io = MockIO::from_slice(&[
            0xC0, 
            0,          // direction
            CommandCode::FlashData as u8,
            20, 0,      // size
            1, 0, 0, 0, // checksum
            4, 0, 0, 0, // size
            3, 0, 0, 0, // sequence_num
            0, 0, 0, 0, // reserved 1
            0, 0, 0, 0, // reserved 1
            9, 8, 7, 6, // payload
            0xC0]);
        let mut stub = Stub::new(&mut io);
        let_assert!( Ok(Command::Data(cmd, data)) = stub.wait_for_command(&mut dummy) );
        assert!( {cmd.base.code == CommandCode::FlashData} );
        assert!( {cmd.base.size == 20} );
        assert!( {cmd.base.checksum == 1} );
        assert!( {cmd.size == 4} );
        assert!( {cmd.sequence_num == 3} );
        assert!( {cmd.reserved[0] == 0} );
        assert!( {cmd.reserved[1] == 0} );
        assert!( data == &[9, 8, 7, 6] );

        // Check Sync command
        let mut io = MockIO::from_slice(&[
            0xC0, 
            0,          // direction
            CommandCode::Sync as u8,
            36, 0,      // size
            1, 0, 0, 0, // checksum
            0x7, 0x7, 0x12, 0x20,
            0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
            0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
            0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
            0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
            0xC0]);
        let mut stub = Stub::new(&mut io);
        let_assert!( Ok(Command::Sync(_)) = stub.wait_for_command(&mut dummy) );

        // Check ReadReg command
        let mut io = MockIO::from_slice(&[
            0xC0, 
            0,          // direction
            CommandCode::ReadReg as u8,
            4, 0,       // size
            1, 0, 0, 0, // checksum
            200, 0, 0, 0, // address
            0xC0]);
        let mut stub = Stub::new(&mut io);
        let_assert!( Ok(Command::ReadReg(address)) = stub.wait_for_command(&mut dummy) );
        assert!( address == 200 );
    }

    #[test]
    fn test_send_response() {
        
        // Can write error response
        let mut io = MockIO::new();
        let mut stub = Stub::new(&mut io);
        let mut response = Response::new(CommandCode::FlashBegin);
        response.error(Error::BadDataChecksum);
        let expected = &[0xC0, 1, CommandCode::FlashBegin as u8, 2,0, 0,0,0,0, 1, Error::BadDataChecksum as u8, 0xC0];
        assert!( stub.send_response(&response) == Ok(()));
        assert!( io.written() == expected);

        // Can write response with data
        let mut io = MockIO::new();
        let mut stub = Stub::new(&mut io);
        let data = &[1, 2, 3, 4, 5, 6, 7, 8];
        let mut response = Response::new(CommandCode::FlashBegin);
        response.data(data);
        let expected = &[0xC0, 1, CommandCode::FlashBegin as u8, 10,0, 0,0,0,0, 0,0, 1, 2, 3, 4, 5, 6, 7, 8, 0xC0];
        assert!( stub.send_response(&response) == Ok(()));
        assert!( io.written() == expected);
    }

    // fn slice_to_cmd(slice: &[u8]) -> Vec<u8> {
    //     let mut v = Vec::new();
    //     v.push(0xC0);
    //     v.extend_from_slice( slice );
    //     v.push(0xC0);
    //     v
    // }

    fn decorate_command<T>(data: T) -> Vec<u8> {
        let mut v = Vec::new();
        v.push(0xC0);
        v.extend_from_slice( unsafe{ to_slice_u8(&data) } );
        v.push(0xC0);
        v
    }

    #[repr(C, packed(1))]
    pub struct TestResponse {
        pub direction: u8,
        pub command: CommandCode,
        pub size: u16,
        pub value: u32,
        pub status: u8,
        pub error: u8,
    }

    #[test]
    fn test_read_register() {
        let cmd = ReadRegCommand {
            base: CommandBase { direction: 1, code: CommandCode::ReadReg, size: 4, checksum: 0 },
            address: 200
        };
        let mut io = MockIO::from_slice(&decorate_command(cmd));
        let mut stub = Stub::new(&mut io);
        
        let ctx = target::read_register_context();
        ctx.expect().with(predicate::eq(200)).returning(|x| x + 1);
        assert!( Ok(()) == stub.process_commands() );

        // let resp = TestResponse {
        //     direction: 1, command: CommandCode::ReadReg, size: 2, value: 201, status: 0, error: 0
        // };
        // let expect = decorate_command(resp);
        let expect = &[0xC0, 1, CommandCode::ReadReg as u8, 2,0, 201,0,0,0, 0,0, 0xC0];
        assert!( io.written() == expect );
    }

    #[test]
    fn test_mock() {
        let mut io = MockIO::new();

        io.fill(&[1, 2, 3]);
        assert!(io.read() == Ok(1));
        assert!(io.read() == Ok(2));
        assert!(io.read() == Ok(3));
        assert!(io.read() == Err(Incomplete));
    }
}

// wait_for_command -> slip::read_packet -> || -> io::read