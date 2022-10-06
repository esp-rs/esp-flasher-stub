#[allow(unused)]
#[repr(u8)]
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum Error {
    None = 0,
    BadDataLen = 0xC0,
    BadDataChecksum = 0xC1,
    BadBlocksize = 0xC2,
    InvalidCommand = 0xC3,
    FailedSpiOp = 0xC4,
    FailedSpiUnlock = 0xC5,
    NotInFlashMode = 0xC6,
    Inflate = 0xC7,
    NotEnoughData = 0xC8,
    TooMuchData = 0xC9,
    CmdNotImplemented = 0xFF,

    Err0x63 = 0x63,
    Err0x32 = 0x32,
    Err0x33 = 0x33,
    Err0x34 = 0x34,
    Err0x35 = 0x35,
}

#[allow(unused)]
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum Code {
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
    GetSecurityInfo = 0x14,
    EraseFlash = 0xD0,
    EraseRegion = 0xD1,
    ReadFlash = 0xD2,
    RunUserCode = 0xD3,
    FlashEncryptedData = 0xD4,
}

#[derive(PartialEq, Copy, Clone, Debug)]
#[repr(C, packed(1))]
pub struct Base {
    direction: Direction,
    pub code: Code,
    pub size: u16,
    pub checksum: u32,
}

#[derive(PartialEq, Copy, Clone, Debug)]
#[repr(C, packed(1))]
pub struct Sync {
    pub base: Base,
    pub payload: [u8; 36],
}

#[derive(PartialEq, Copy, Clone, Debug)]
#[repr(C, packed(1))]
pub struct Begin {
    pub base: Base,
    pub total_size: u32,
    pub packt_count: u32,
    pub packet_size: u32,
    pub offset: u32,
}

#[derive(PartialEq, Copy, Clone, Debug)]
#[repr(C, packed(1))]
pub struct Data {
    pub base: Base,
    pub size: u32,
    pub sequence_num: u32,
    pub reserved: [u32; 2],
}

#[derive(PartialEq, Copy, Clone)]
#[repr(C, packed(1))]
pub struct End {
    pub base: Base,
    pub run_user_code: u32,
}

#[derive(PartialEq, Copy, Clone)]
#[repr(C, packed(1))]
pub struct WriteReg {
    pub base: Base,
    pub address: u32,
    pub value: u32,
    pub mask: u32,
    pub delay_us: u32,
}

#[derive(PartialEq, Copy, Clone)]
#[repr(C, packed(1))]
pub struct ReadReg {
    pub base: Base,
    pub address: u32,
}

// Possibly move to other module
#[derive(PartialEq, Eq, Copy, Clone)]
#[repr(C, packed(1))]
pub struct SpiParams {
    pub id: u32,
    pub total_size: u32,
    pub block_size: u32,
    pub sector_size: u32,
    pub page_size: u32,
    pub status_mask: u32,
}

#[derive(PartialEq, Copy, Clone)]
#[repr(C, packed(1))]
pub struct SpiSetParams {
    pub base: Base,
    pub params: SpiParams,
}

#[derive(PartialEq, Copy, Clone)]
#[repr(C, packed(1))]
pub struct ChangeBaudrate {
    pub base: Base,
    pub new: u32,
    pub old: u32,
}

#[derive(PartialEq, Copy, Clone)]
#[repr(C, packed(1))]
pub struct SpiFlashMd5 {
    pub base: Base,
    pub address: u32,
    pub size: u32,
    pub reserved: [u32; 2],
}

#[derive(PartialEq, Copy, Clone)]
#[repr(C, packed(1))]
pub struct EraseRegion {
    pub base: Base,
    pub address: u32,
    pub size: u32,
}

// Possibly move to other module
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
#[repr(C, packed(1))]
pub struct ReadFlashParams {
    pub address: u32,
    pub total_size: u32,
    pub packet_size: u32,
    pub max_inflight: u32,
}

#[derive(PartialEq, Copy, Clone, Debug)]
#[repr(C, packed(1))]
pub struct ReadFlash {
    pub base: Base,
    pub params: ReadFlashParams,
}

#[allow(unused)]
#[repr(u8)]
#[derive(PartialEq, Copy, Clone, Debug)]
enum Direction {
    In,
    Out,
}

#[repr(u8)]
#[derive(PartialEq, Copy, Clone)]
enum Status {
    Success,
    Failure,
}

#[derive(PartialEq, Copy, Clone)]
#[repr(C, packed(1))]
pub struct Response<'a> {
    direction: Direction,
    command: Code,
    size: u16,
    value: u32,
    status: Status,
    error: Error,
    pub data: &'a [u8],
}

// Size of sesponse without data reference
pub const RESPONSE_SIZE: usize = 10;

impl<'a> Response<'a> {
    pub fn new(cmd: Code) -> Self {
        Response {
            direction: Direction::Out,
            command: cmd,
            size: 2,
            value: 0,
            status: Status::Success,
            error: Error::None,
            data: &[],
        }
    }

    pub fn value(&mut self, value: u32) {
        self.value = value;
    }

    #[allow(unused)]
    pub fn data(&mut self, data: &'a [u8]) {
        self.size = 2 + data.len() as u16;
        self.data = data;
    }

    pub fn error(&mut self, error: Error) {
        self.status = Status::Failure;
        self.error = error;
    }
}
