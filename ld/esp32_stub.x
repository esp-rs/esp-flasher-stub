
/* before memory.x to allow override */
ENTRY(Reset)

/* This memory map assumes the flash cache is on; 
   the blocks used are excluded from the various memory ranges 
   
   see: https://github.com/espressif/esp-idf/blob/5b1189570025ba027f2ff6c2d91f6ffff3809cc2/components/heap/port/esp32/memory_layout.c
   for details
   */

/* override entry point */
ENTRY(ESP32Reset)

/* reserved at the start of DRAM for e.g. the BT stack */
RESERVE_DRAM = 0x0;

/* reserved at the start of the RTC memories for use by the ULP processor */
RESERVE_RTC_FAST = 0;
RESERVE_RTC_SLOW = 0;

/* define stack size for both cores */
STACK_SIZE = 32k;

/* Specify main memory areas */
MEMORY
{
  reserved_cache_seg     : ORIGIN = 0x40070000, len = 64k /* SRAM0; reserved for usage as flash cache*/
  iram_seg ( RX )        : ORIGIN = 0x40080000, len = 128k /* SRAM0 */

  reserved_for_rom_seg   : ORIGIN = 0x3FFAE000, len = 8k /* SRAM2; reserved for usage by the ROM */
  dram_seg ( RW )        : ORIGIN = 0x3FFB0000 + RESERVE_DRAM, len = 176k - RESERVE_DRAM /* SRAM2+1; first 64kB used by BT if enable */
  reserved_for_boot_seg  : ORIGIN = 0x3FFDC200, len = 144k /* SRAM1; reserved for static ROM usage; can be used for heap */

  /* external flash 
     The 0x20 offset is a convenience for the app binary image generation.
     Flash cache has 64KB pages. The .bin file which is flashed to the chip
     has a 0x18 byte file header, and each segment has a 0x08 byte segment
     header. Setting this offset makes it simple to meet the flash cache MMU's
     constraint that (paddr % 64KB == vaddr % 64KB).)
  */
  irom_seg ( RX )        : ORIGIN = 0x400D0020, len = 3M - 0x20
  drom_seg ( R )         : ORIGIN = 0x3F400020, len = 4M - 0x20

  /* RTC fast memory (executable). Persists over deep sleep. Only for core 0 (PRO_CPU) */
  rtc_fast_iram_seg(RWX) : ORIGIN = 0x400C0000, len = 8k

  /* RTC fast memory (same block as above), viewed from data bus. Only for core 0 (PRO_CPU) */
  rtc_fast_dram_seg(RW)  : ORIGIN = 0x3FF80000 + RESERVE_RTC_FAST, len = 8k - RESERVE_RTC_FAST

  /* RTC slow memory (data accessible). Persists over deep sleep. */
  rtc_slow_seg(RW)       : ORIGIN = 0x50000000 + RESERVE_RTC_SLOW, len = 8k - RESERVE_RTC_SLOW
}

/* map generic regions to output sections */
REGION_ALIAS("ROTEXT", irom_seg);
REGION_ALIAS("RWTEXT", iram_seg);
REGION_ALIAS("RODATA", drom_seg);
REGION_ALIAS("RWDATA", dram_seg);

/* esp32 specific regions */
SECTIONS {
  .rtc_fast.text : {
   . = ALIGN(4);
    *(.rtc_fast.literal .rtc_fast.text .rtc_fast.literal.* .rtc_fast.text.*)
  } > rtc_fast_iram_seg AT > RODATA

  /*
    This section is required to skip rtc.text area because rtc_iram_seg and
    rtc_data_seg are reflect the same address space on different buses.
  */
  .rtc_fast.dummy (NOLOAD) :
  {
    _rtc_dummy_start = ABSOLUTE(.); /* needed to make section proper size */
    . = SIZEOF(.rtc_fast.text);
    _rtc_dummy_end = ABSOLUTE(.); /* needed to make section proper size */
  } > rtc_fast_dram_seg
  
  
  .rtc_fast.data :
  {
    . = ALIGN(4);
    _rtc_fast_data_start = ABSOLUTE(.);
    *(.rtc_fast.data .rtc_fast.data.*)
    _rtc_fast_data_end = ABSOLUTE(.);
  } > rtc_fast_dram_seg AT > RODATA

 .rtc_fast.bss (NOLOAD) :
  {
    . = ALIGN(4);
    _rtc_fast_bss_start = ABSOLUTE(.);
    *(.rtc_fast.bss .rtc_fast.bss.*)
    _rtc_fast_bss_end = ABSOLUTE(.);
  } > rtc_fast_dram_seg

 .rtc_fast.noinit (NOLOAD) :
  {
    . = ALIGN(4);
    *(.rtc_fast.noinit .rtc_fast.noinit.*)
  } > rtc_fast_dram_seg


 .rtc_slow.text : {
   . = ALIGN(4);
    *(.rtc_slow.literal .rtc_slow.text .rtc_slow.literal.* .rtc_slow.text.*)
  } > rtc_slow_seg AT > RODATA

  .rtc_slow.data :
  {
    . = ALIGN(4);
    _rtc_slow_data_start = ABSOLUTE(.);
    *(.rtc_slow.data .rtc_slow.data.*)
    _rtc_slow_data_end = ABSOLUTE(.);
  } > rtc_slow_seg AT > RODATA

 .rtc_slow.bss (NOLOAD) :
  {
    . = ALIGN(4);
    _rtc_slow_bss_start = ABSOLUTE(.);
    *(.rtc_slow.bss .rtc_slow.bss.*)
    _rtc_slow_bss_end = ABSOLUTE(.);
  } > rtc_slow_seg

 .rtc_slow.noinit (NOLOAD) :
  {
    . = ALIGN(4);
    *(.rtc_slow.noinit .rtc_slow.noinit.*)
  } > rtc_slow_seg

  .text : ALIGN(4)
  {
    /* Vector table */
    . = 0x0;
    _init_start = ABSOLUTE(.);
    . = 0x00000000 ;
    KEEP(*(.WindowOverflow4.text));
    . = 0x00000040;
    KEEP(*(.WindowUnderflow4.text));
    . = 0x00000080;
    KEEP(*(.WindowOverflow8.text));
    . = 0x000000C0;
    KEEP(*(.WindowUnderflow8.text));
    . = 0x00000100;
    KEEP(*(.WindowOverflow12.text));
    . = 0x00000140;
    KEEP(*(.WindowUnderflow12.text));
    . = 0x00000180;
    KEEP(*(.Level2InterruptVector.text));
    . = 0x000001C0;
    KEEP(*(.Level3InterruptVector.text));
    . = 0x00000200;
    KEEP(*(.Level4InterruptVector.text));
    . = 0x00000240;
    KEEP(*(.Level5InterruptVector.text));
    . = 0x00000280;
    KEEP(*(.DebugExceptionVector.text));
    . = 0x000002C0;
    KEEP(*(.NMIExceptionVector.text));
    . = 0x00000300;
    KEEP(*(.KernelExceptionVector.text));
    . = 0x00000340;
    KEEP(*(.UserExceptionVector.text));
    . = 0x000003C0;
    KEEP(*(.DoubleExceptionVector.text));
    . = 0x400;
    _init_end = ABSOLUTE(.);

    _stext = .;
    . = ALIGN (4);
    _text_start = ABSOLUTE(.);
    . = ALIGN (4);
    *(.literal .text .literal.* .text.*)
    _text_end = ABSOLUTE(.);
    _etext = .;
    *(.rwtext.literal .rwtext .rwtext.literal.* .rwtext.*)
  } > RWTEXT

  .data : ALIGN(4)
  {
    _data_start = ABSOLUTE(.);
    . = ALIGN (4);
    *(.data .data.*)
    *(.rodata .rodata.*)
    _data_end = ABSOLUTE(.);
  } > RWDATA AT > RWTEXT

  /* LMA of .data */
  _sidata = LOADADDR(.data);

  .bss (NOLOAD) : ALIGN(4)
  {
    _bss_start = ABSOLUTE(.);
    . = ALIGN (4);
    *(.bss .bss.* COMMON)
    _bss_end = ABSOLUTE(.);
  } > RWDATA

  .noinit (NOLOAD) : ALIGN(4)
  {
    . = ALIGN(4);
    *(.noinit .noinit.*)
  } > RWDATA

 /* must be last segment using RWTEXT */
  .text_heap_start (NOLOAD) : ALIGN(4)
  {
    . = ALIGN (4);
    _text_heap_start = ABSOLUTE(.);
  } > RWTEXT

 /* must be last segment using RWDATA */
  .heap_start (NOLOAD) : ALIGN(4)
  {
    . = ALIGN (4);
    _heap_start = ABSOLUTE(.);
  } > RWDATA
} 

_heap_end = ABSOLUTE(ORIGIN(dram_seg))+LENGTH(dram_seg)+LENGTH(reserved_for_boot_seg) - 2*STACK_SIZE;
_text_heap_end = ABSOLUTE(ORIGIN(iram_seg)+LENGTH(iram_seg));
_external_heap_end = ABSOLUTE(ORIGIN(psram_seg)+LENGTH(psram_seg));

_stack_start_cpu1 = _heap_end;
_stack_end_cpu1 = _stack_start_cpu1 + STACK_SIZE;
_stack_start_cpu0 = _stack_end_cpu1;
_stack_end_cpu0 = _stack_start_cpu0 + STACK_SIZE;

EXTERN(DefaultHandler);

EXTERN(WIFI_EVENT); /* Force inclusion of WiFi libraries */

PROVIDE(WIFI_MAC = DefaultHandler);
PROVIDE(WIFI_NMI = DefaultHandler);
PROVIDE(WIFI_BB = DefaultHandler);
PROVIDE(BT_MAC = DefaultHandler);
PROVIDE(BT_BB = DefaultHandler);
PROVIDE(BT_BB_NMI = DefaultHandler);
PROVIDE(RWBT = DefaultHandler);
PROVIDE(RWBLE = DefaultHandler);
PROVIDE(RWBT_NMI = DefaultHandler);
PROVIDE(RWBLE_NMI = DefaultHandler);
PROVIDE(UHCI0 = DefaultHandler);
PROVIDE(UHCI1 = DefaultHandler);
PROVIDE(TG0_T0_LEVEL = DefaultHandler);
PROVIDE(TG0_T1_LEVEL = DefaultHandler);
PROVIDE(TG0_WDT_LEVEL = DefaultHandler);
PROVIDE(TG0_LACT_LEVEL = DefaultHandler);
PROVIDE(TG1_T0_LEVEL = DefaultHandler);
PROVIDE(TG1_T1_LEVEL = DefaultHandler);
PROVIDE(TG1_WDT_LEVEL = DefaultHandler);
PROVIDE(TG1_LACT_LEVEL = DefaultHandler);
PROVIDE(GPIO = DefaultHandler);
PROVIDE(GPIO_NMI = DefaultHandler);
PROVIDE(SPI0 = DefaultHandler);
PROVIDE(SPI1 = DefaultHandler);
PROVIDE(SPI2 = DefaultHandler);
PROVIDE(SPI3 = DefaultHandler);
PROVIDE(I2S0 = DefaultHandler);
PROVIDE(I2S1 = DefaultHandler);
PROVIDE(UART0 = DefaultHandler);
PROVIDE(UART1 = DefaultHandler);
PROVIDE(UART2 = DefaultHandler);
PROVIDE(PWM0 = DefaultHandler);
PROVIDE(PWM1 = DefaultHandler);
PROVIDE(LEDC = DefaultHandler);
PROVIDE(EFUSE = DefaultHandler);
PROVIDE(TWAI = DefaultHandler);
PROVIDE(RTC_CORE = DefaultHandler);
PROVIDE(RMT = DefaultHandler);
PROVIDE(PCNT = DefaultHandler);
PROVIDE(I2C_EXT0 = DefaultHandler);
PROVIDE(I2C_EXT1 = DefaultHandler);
PROVIDE(RSA = DefaultHandler);
PROVIDE(SPI1_DMA = DefaultHandler);
PROVIDE(SPI2_DMA = DefaultHandler);
PROVIDE(SPI3_DMA = DefaultHandler);
PROVIDE(TIMER1 = DefaultHandler);
PROVIDE(TIMER2 = DefaultHandler);
PROVIDE(TG0_T0_EDGE = DefaultHandler);
PROVIDE(TG0_T1_EDGE = DefaultHandler);
PROVIDE(TG0_WDT_EDGE = DefaultHandler);
PROVIDE(TG0_LACT_EDGE = DefaultHandler);
PROVIDE(TG1_T0_EDGE = DefaultHandler);
PROVIDE(TG1_T1_EDGE = DefaultHandler);
PROVIDE(TG1_WDT_EDGE = DefaultHandler);
PROVIDE(TG1_LACT_EDGE = DefaultHandler);


/* after memory.x to allow override */
PROVIDE(__pre_init = DefaultPreInit);
PROVIDE(__zero_bss = default_mem_hook);
PROVIDE(__init_data = default_mem_hook);

/* exception vector for the ESP32, requiring high priority interrupts and register window support */

/* high level exception/interrupt routines, which can be override with Rust functions */
PROVIDE(__exception = __default_exception);
PROVIDE(__user_exception = __default_user_exception);
PROVIDE(__double_exception = __default_double_exception);
PROVIDE(__level_1_interrupt = __default_interrupt);
PROVIDE(__level_2_interrupt = __default_interrupt);
PROVIDE(__level_3_interrupt = __default_interrupt);
PROVIDE(__level_4_interrupt = __default_interrupt);
PROVIDE(__level_5_interrupt = __default_interrupt);
PROVIDE(__level_6_interrupt = __default_interrupt);
PROVIDE(__level_7_interrupt = __default_interrupt);

/* high level CPU interrupts */
PROVIDE(Timer0 = __default_user_exception);
PROVIDE(Timer1 = __default_user_exception);
PROVIDE(Timer2 = __default_user_exception);
PROVIDE(Timer3 = __default_user_exception);
PROVIDE(Profiling = __default_user_exception);
PROVIDE(NMI = __default_user_exception);
PROVIDE(Software0 = __default_user_exception);
PROVIDE(Software1 = __default_user_exception);

/* low level exception/interrupt, which must be overridden using naked functions */
PROVIDE(__naked_user_exception = __default_naked_exception);
PROVIDE(__naked_kernel_exception = __default_naked_exception);
PROVIDE(__naked_double_exception = __default_naked_double_exception);
PROVIDE(__naked_level_2_interrupt = __default_naked_level_2_interrupt);
PROVIDE(__naked_level_3_interrupt = __default_naked_level_3_interrupt);
PROVIDE(__naked_level_4_interrupt = __default_naked_level_4_interrupt);
PROVIDE(__naked_level_5_interrupt = __default_naked_level_5_interrupt);
PROVIDE(__naked_level_6_interrupt = __default_naked_level_6_interrupt);
PROVIDE(__naked_level_7_interrupt = __default_naked_level_7_interrupt);

PROVIDE(level1_interrupt = DefaultHandler);
PROVIDE(level2_interrupt = DefaultHandler);
PROVIDE(level3_interrupt = DefaultHandler);
PROVIDE(level4_interrupt = DefaultHandler);
PROVIDE(level5_interrupt = DefaultHandler);
PROVIDE(level6_interrupt = DefaultHandler);
PROVIDE(level7_interrupt = DefaultHandler);

/* needed to force inclusion of the vectors */
EXTERN(__default_exception);
EXTERN(__default_double_exception);
EXTERN(__default_interrupt);

EXTERN(__default_naked_exception);
EXTERN(__default_naked_double_exception);
EXTERN(__default_naked_level_2_interrupt);
EXTERN(__default_naked_level_3_interrupt);
EXTERN(__default_naked_level_4_interrupt);
EXTERN(__default_naked_level_5_interrupt);
EXTERN(__default_naked_level_6_interrupt);
EXTERN(__default_naked_level_7_interrupt);
