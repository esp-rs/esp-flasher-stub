#![allow(dead_code)]

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

type Buffer = heapless::Vec<u8, 1024>;

mod stub
{
    use super::*;
    use super::slip::read_packet;
    // use heapless::Vec;
    use core::mem;
    
    #[derive(PartialEq, Copy, Clone, Debug)]
    pub enum CommandType
    {
        FlashBegin = 0x02,
        FlashData = 0x03,
        FlashEnd = 0x04,
        MemBegin = 0x05,
        MemEnd = 0x06,
        MemData = 0x07,
        Sync = 0x08,
        WriteReg = 0x09,
        ReadReg = 0x0A,
        SpiSetParams = 0x0B,
        SpiAttach = 0x0D,
        ChangeBaudrate = 0x0F,
        FlashDeflBegin = 0x10,
        FlashDeflData = 0x11,
        FlashDeflEnd = 0x12,
        SpiFlashMd5 = 0x13,
        EraseFlash = 0xD0,
        EraseRegion = 0xD1,
        ReadFlash = 0xD2,
        RunUserCode = 0xD3,
    }
    
    #[derive(PartialEq)]
    pub enum Error
    {
        InvalidMessageReceived = 0x5,
        FailedToProcessCommand = 0x6,
        InvalidCRC = 0x7,
        FlashWrite = 0x8,
        FlashRead = 0x9,
        FlashReadLength = 0xA,
        Deflate = 0xB,
    }

    #[derive(PartialEq, Copy, Clone, Debug)]
    #[repr(C, packed(1))]
    pub struct CommandBase {
        pub direction: u8,
        pub command: CommandType,
        pub size: u16,
        pub checksum: u32,
    }
    
    #[derive(PartialEq, Copy, Clone, Debug)]
    #[repr(C, packed(1))]
    pub struct FlashBeginCommand {
        pub base: CommandBase,
        pub erase_addr: u32,
        pub packt_count: u32,
        pub packet_size: u32,
        pub offset: u32,
    }
    
    #[derive(PartialEq, Copy, Clone)]
    #[repr(C, packed(1))]
    pub struct FlashDataCommand<'a> {
        pub base: CommandBase,
        pub size: u32,
        pub sequence_num: u32,
        pub reserved0: u32,
        pub reserved1: u32,
        pub data: &'a [u8],
    }
    
    #[derive(PartialEq, Copy, Clone)]
    #[repr(C, packed(1))]
    pub struct FlashEndCommand {
        pub base: CommandBase,
        pub run_user_code: u32,
    }
    
    #[derive(PartialEq, Copy, Clone)]
    pub enum Command<'a>
    {
        FlashBegin(FlashBeginCommand),
        FlashData(FlashDataCommand<'a>),
        FlashEnd(FlashEndCommand),
    }   

    pub struct Stub<'a> {
        io: &'a mut (dyn InputIO + 'a),
        payload: Buffer,
    }

    impl From<ErrorIO> for Error {
        fn from(_: ErrorIO) -> Self {
            Error::InvalidMessageReceived
        }
    }
    
    // todo: get rid of SIZE
    fn slice_to_struct<T: Sized, const SIZE: usize>(slice: &[u8]) -> T
    {
        let array: [u8; SIZE] = slice.try_into().unwrap();
        unsafe { mem::transmute_copy::<[u8; SIZE], T>(&array) }
    }

    macro_rules! slice_2_struct {
        ($slice:expr, $type:ty) => {
            slice_to_struct::<$type, { mem::size_of::<$type>() }>($slice)
        };
    }
    

    impl<'a> Stub<'a> {

        pub fn new(input_io: &'a mut dyn InputIO) -> Self {
            Stub { 
                io: input_io,
                payload: heapless::Vec::new(),
            }
        }

        pub fn wait_for_command(&mut self) -> Result<Command, Error>
        {
            read_packet(self.io, &mut self.payload)?;

            if self.payload.len() < mem::size_of::<CommandBase>() {
                return Err(Error::InvalidMessageReceived);
            }

            let command = slice_2_struct!(self.payload.as_slice(), FlashBeginCommand);
            Ok(Command::FlashBegin(command))
        }
        
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
        io.write(&[0xC0])?;
        for byte in packet {
            match byte {
                0xC0  => io.write(&[0xDB, 0xDC])?,
                0xDB  => io.write(&[0xDB, 0xDD])?,
                other => io.write(&[*other])?,
            }
        }
        io.write(&[0xC0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::ErrorIO::*;
    use super::stub::*;
    use super::stub::Error::*;
    use super::slip::{read_packet, write_packet};
    use assert2::{assert, let_assert};
    // use matches::assert_matches;
    use std::collections::VecDeque;
    use std::vec::Vec;

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

        // 0xC0 is added at the beginning and end of payload
        assert!( write_packet(&mut io, &[1, 2, 3]) == Ok(()));
        assert!( io.written() == &[0xC0, 1, 2, 3, 0xC0] );
        io.clear();

        // 0xC0 is replaced with 0xDB 0xDC
        assert!( write_packet(&mut io, &[1, 0xC0, 3]) == Ok(()));
        assert!( io.written() == &[0xC0, 1, 0xDB, 0xDC, 3, 0xC0] );
        io.clear();

        // 0xDB is replaced with 0xDB 0xDD
        assert!( write_packet(&mut io, &[1, 0xDB, 3]) == Ok(()));
        assert!( io.written() == &[0xC0, 1, 0xDB, 0xDD, 3, 0xC0] );
        io.clear();
    }

    #[test]
    fn test_wait_for_packet() {
        
        // Returns InvalidMessageReceived after receiving incomplete message
        let mut io = MockIO::from_slice(&[
            0xC0, 
            1,
            CommandType::FlashBegin as u8,
            4, 0,
            1, 0, 0, 0, // checksum
            2, 0, 0, 0, // erase_addr
            3, 0, 0, 0, // packt_count
            4, 0, 0, 0, // packet_size
            5, 0, 0, 0, // offset
            0xC0]);
        let mut stub = Stub::new(&mut io);
        let_assert!( Ok(Command::FlashBegin(cmd)) = stub.wait_for_command() );
        println!("{:?}", cmd);
        assert!(cmd.base.direction == 1);

        // let _slice = [1 as u8, 0xAu8, 16u16];
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