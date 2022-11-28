// Size of sesponse without data reference
pub const RESPONSE_SIZE: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
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

    EraseErr = 0x36, //TODO: Is it OK to add custom Error?
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCode {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed(1))]
pub struct CommandBase {
    pub direction: u8,
    pub code: CommandCode,
    pub size: u16,
    pub checksum: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed(1))]
pub struct SyncCommand {
    pub base: CommandBase,
    pub payload: [u8; 36],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed(1))]
pub struct BeginCommand {
    pub base: CommandBase,
    pub total_size: u32,
    pub packt_count: u32,
    pub packet_size: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed(1))]
pub struct DataCommand {
    pub base: CommandBase,
    pub size: u32,
    pub sequence_num: u32,
    pub reserved: [u32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed(1))]
pub struct EndFlashCommand {
    pub base: CommandBase,
    pub run_user_code: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed(1))]
pub struct MemEndCommand {
    pub base: CommandBase,
    pub stay_in_stub: u32,
    pub entrypoint: fn(),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed(1))]
pub struct WriteRegCommand {
    pub base: CommandBase,
    pub address: u32,
    pub value: u32,
    pub mask: u32,
    pub delay_us: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed(1))]
pub struct ReadRegCommand {
    pub base: CommandBase,
    pub address: u32,
}

// Possibly move to other module
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed(1))]
pub struct SpiParams {
    pub id: u32,
    pub total_size: u32,
    pub block_size: u32,
    pub sector_size: u32,
    pub page_size: u32,
    pub status_mask: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed(1))]
pub struct SpiSetParamsCommand {
    pub base: CommandBase,
    pub params: SpiParams,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed(1))]
pub struct ChangeBaudrateCommand {
    pub base: CommandBase,
    pub new: u32,
    pub old: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed(1))]
pub struct SpiFlashMd5Command {
    pub base: CommandBase,
    pub address: u32,
    pub size: u32,
    pub reserved: [u32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed(1))]
pub struct EraseRegionCommand {
    pub base: CommandBase,
    pub address: u32,
    pub size: u32,
}

// Possibly move to other module
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed(1))]
pub struct ReadFlashParams {
    pub address: u32,
    pub total_size: u32,
    pub packet_size: u32,
    pub max_inflight: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed(1))]
pub struct ReadFlashCommand {
    pub base: CommandBase,
    pub params: ReadFlashParams,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed(1))]
pub struct Response<'a> {
    pub direction: u8,
    pub command: CommandCode,
    pub size: u16,
    pub value: u32,
    pub status: u8,
    pub error: u8,
    pub data: &'a [u8],
}

impl<'a> Response<'a> {
    pub fn new(cmd: CommandCode) -> Self {
        Response {
            direction: 1,
            command: cmd,
            size: 2,
            value: 0,
            status: 0,
            error: 0,
            data: &[],
        }
    }

    pub fn value(&mut self, value: u32) {
        self.value = value;
    }

    pub fn data(&mut self, data: &'a [u8]) {
        self.size = 2 + data.len() as u16;
        self.data = data;
    }

    pub fn error(&mut self, error: Error) {
        self.status = 1;
        self.error = error as u8;
    }
}
