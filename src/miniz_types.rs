const TINFL_MAX_HUFF_TABLES: usize = 3;
const TINFL_MAX_HUFF_SYMBOLS_0: usize = 288;
const TINFL_MAX_HUFF_SYMBOLS_1: usize = 32;
const TINFL_MAX_HUFF_SYMBOLS_2: usize = 19;
const TINFL_FAST_LOOKUP_BITS: usize = 10;
const TINFL_FAST_LOOKUP_SIZE: usize = 1 << TINFL_FAST_LOOKUP_BITS;

pub const TINFL_FLAG_PARSE_ZLIB_HEADER: u32 = 1;
pub const TINFL_FLAG_HAS_MORE_INPUT: u32 = 2;
pub const TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF: u32 = 4;
pub const TINFL_FLAG_COMPUTE_ADLER32: u32 = 8;

#[repr(C)]
#[derive(PartialEq, PartialOrd)]
pub enum TinflStatus {
    FailedCannotMakeProgress = -4,
    BadParam = -3,
    Adler32Mismatch = -2,
    Failed = -1,
    Done = 0,
    NeedsMoreInput = 1,
    HasMoreOutput = 2
}

#[derive(Clone, Copy)]
#[repr(C, packed(1))]
struct tinfl_huff_table
{
    code_size: [u8; TINFL_MAX_HUFF_SYMBOLS_0],
    look_up: [u16;TINFL_FAST_LOOKUP_SIZE],
    tree: [u16; TINFL_MAX_HUFF_SYMBOLS_0 * 2]
}

#[repr(C, packed(1))]
pub struct tinfl_decompressor
{
    pub state: u32,
    num_bits: u32,
    zhdr0: u32,
    zhdr1: u32,
    z_adler32: u32,
    m_final: u32,
    m_type: u32,
    check_adler32: u32,
    dist: u32,
    counter: u32,
    num_extra: u32,
    table_sizes: [u32; TINFL_MAX_HUFF_TABLES],
    bit_buf: u64,
    dist_from_out_buf_start: u32,
    tables: [tinfl_huff_table; TINFL_MAX_HUFF_TABLES],
    raw_header: [u8; 4],
    len_codes: [u8; TINFL_MAX_HUFF_SYMBOLS_0 + TINFL_MAX_HUFF_SYMBOLS_1 + 137]
}

impl Default for tinfl_huff_table {
    fn default() -> Self {
        tinfl_huff_table {
            code_size: [0; TINFL_MAX_HUFF_SYMBOLS_0],
            look_up: [0;TINFL_FAST_LOOKUP_SIZE],
            tree: [0; TINFL_MAX_HUFF_SYMBOLS_0 * 2]
        }
    }
}

impl Default for tinfl_decompressor {
    fn default() -> Self {
        tinfl_decompressor {
            state: 0,
            num_bits: 0,
            zhdr0: 0,
            zhdr1: 0,
            z_adler32: 0,
            m_final: 0,
            m_type: 0,
            check_adler32: 0,
            dist: 0,
            counter: 0,
            num_extra: 0,
            table_sizes: [0; TINFL_MAX_HUFF_TABLES],
            bit_buf: 0,
            dist_from_out_buf_start: 0,
            tables: [Default::default(); TINFL_MAX_HUFF_TABLES],
            raw_header: [0; 4],
            len_codes: [0; TINFL_MAX_HUFF_SYMBOLS_0 + TINFL_MAX_HUFF_SYMBOLS_1 + 137]
        }
    }
}