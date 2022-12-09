/* before memory.x to allow override */
ENTRY(Reset)

/* This memory map assumes the flash cache is on; 
   the blocks used are excluded from the various memory ranges 
   
   see: https://github.com/espressif/esp-idf/blob/5b1189570025ba027f2ff6c2d91f6ffff3809cc2/components/heap/port/esp32s2/memory_layout.c
   for details
   */

/* override entry point */
ENTRY(ESP32Reset)

/* reserved at the start of DRAM/IRAM */
RESERVE_CACHES = 0x2000;

VECTORS_SIZE = 0x400;

/* reserved at the start of the RTC memories for use by the ULP processor */
RESERVE_RTC_FAST = 0;
RESERVE_RTC_SLOW = 0;

/* define stack size for both cores */
STACK_SIZE = 8k;

/* Specify main memory areas */
MEMORY
{
  vectors_seg ( RX )     : ORIGIN = 0x40020000 + RESERVE_CACHES, len = VECTORS_SIZE /* SRAM0 */
  iram_seg ( RX )        : ORIGIN = 0x40020000 + RESERVE_CACHES + VECTORS_SIZE, len = 192k - RESERVE_CACHES - VECTORS_SIZE /* SRAM0 */

  dram_seg ( RW )        : ORIGIN = 0x3FFB0000 + RESERVE_CACHES + VECTORS_SIZE, len = 192k - RESERVE_CACHES - VECTORS_SIZE

  /* SRAM1; reserved for static ROM usage; can be used for heap.
     Length based on the "_dram0_rtos_reserved_start" symbol from IDF used to delimit the
     ROM data reserved region:
     https://github.com/espressif/esp-idf/blob/bcb34ca7aef4e8d3b97d75ad069b960fb1c17c16/components/heap/port/esp32s2/memory_layout.c#L121-L122
  */
  reserved_for_boot_seg  : ORIGIN = 0x3FFE0000, len = 0x1FA10

  /* external flash 
     The 0x20 offset is a convenience for the app binary image generation.
     Flash cache has 64KB pages. The .bin file which is flashed to the chip
     has a 0x18 byte file header, and each segment has a 0x08 byte segment
     header. Setting this offset makes it simple to meet the flash cache MMU's
     constraint that (paddr % 64KB == vaddr % 64KB).)
  */
  irom_seg ( RX )        : ORIGIN = 0x40080020, len = 3M - 0x20
  drom_seg ( R )         : ORIGIN = 0x3F000020, len = 4M - 0x20


  /* RTC fast memory (executable). Persists over deep sleep. Only for core 0 (PRO_CPU) */
  rtc_fast_iram_seg(RWX) : ORIGIN = 0x40070000, len = 8k

  /* RTC fast memory (same block as above), viewed from data bus. Only for core 0 (PRO_CPU) */
  rtc_fast_dram_seg(RW)  : ORIGIN = 0x3ff9e000 + RESERVE_RTC_FAST, len = 8k - RESERVE_RTC_FAST

  /* RTC slow memory (data accessible). Persists over deep sleep. */
  rtc_slow_seg(RW)       : ORIGIN = 0x50000000 + RESERVE_RTC_SLOW, len = 8k - RESERVE_RTC_SLOW

  /* external memory, including data and text */
  psram_seg(RWX)         : ORIGIN =  0x3F500000, len = 0xA80000
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
} 

_external_ram_start = ABSOLUTE(ORIGIN(psram_seg));
_external_ram_end = ABSOLUTE(ORIGIN(psram_seg)+LENGTH(psram_seg));

_heap_end = ABSOLUTE(ORIGIN(dram_seg))+LENGTH(dram_seg)+LENGTH(reserved_for_boot_seg) - STACK_SIZE;
_text_heap_end = ABSOLUTE(ORIGIN(iram_seg)+LENGTH(iram_seg));
_external_heap_end = ABSOLUTE(ORIGIN(psram_seg)+LENGTH(psram_seg));

_stack_start_cpu0 = _heap_end;
_stack_end_cpu0 = _stack_start_cpu0 + STACK_SIZE;

EXTERN(DefaultHandler);

PROVIDE(WIFI_MAC = DefaultHandler);
PROVIDE(WIFI_MAC_NMI = DefaultHandler);
PROVIDE(WIFI_PWR = DefaultHandler);
PROVIDE(WIFI_BB = DefaultHandler);
PROVIDE(BT_MAC = DefaultHandler);
PROVIDE(BT_BB = DefaultHandler);
PROVIDE(BT_BB_NMI = DefaultHandler);
PROVIDE(RWBT = DefaultHandler);
PROVIDE(RWBLE = DefaultHandler);
PROVIDE(RWBT_NMI = DefaultHandler);
PROVIDE(RWBLE_NMI = DefaultHandler);
PROVIDE(UHCI0 = DefaultHandler);
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
PROVIDE(DEDICATED_GPIO = DefaultHandler);
PROVIDE(SPI1 = DefaultHandler);
PROVIDE(SPI2 = DefaultHandler);
PROVIDE(SPI3 = DefaultHandler);
PROVIDE(UART0 = DefaultHandler);
PROVIDE(UART1 = DefaultHandler);
PROVIDE(LEDC = DefaultHandler);
PROVIDE(EFUSE = DefaultHandler);
PROVIDE(TWAI = DefaultHandler);
PROVIDE(RTC_CORE = DefaultHandler);
PROVIDE(RMT = DefaultHandler);
PROVIDE(PCNT = DefaultHandler);
PROVIDE(I2C_EXT0 = DefaultHandler);
PROVIDE(I2C_EXT1 = DefaultHandler);
PROVIDE(RSA = DefaultHandler);
PROVIDE(SHA = DefaultHandler);
PROVIDE(AES = DefaultHandler);
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
PROVIDE(SYSTIMER_TARGET0 = DefaultHandler);
PROVIDE(SYSTIMER_TARGET1 = DefaultHandler);
PROVIDE(SYSTIMER_TARGET2 = DefaultHandler);
PROVIDE(APB_ADC = DefaultHandler);

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


/* Define output sections */
SECTIONS {

  .text : ALIGN(4)
  {
    . = 0x0;
    _init_start = ABSOLUTE(.);
    . = 0;
    KEEP(*(.WindowOverflow4.text));
    . = 64;
    KEEP(*(.WindowUnderflow4.text));
    . = 128;
    KEEP(*(.WindowOverflow8.text));
    . = 192;
    KEEP(*(.WindowUnderflow8.text));
    . = 256;
    KEEP(*(.WindowOverflow12.text));
    . = 320;
    KEEP(*(.WindowUnderflow12.text));
    . = 384;
    KEEP(*(.Level2InterruptVector.text));
    . = 448;
    KEEP(*(.Level3InterruptVector.text));
    . = 512;
    KEEP(*(.Level4InterruptVector.text));
    . = 576;
    KEEP(*(.Level5InterruptVector.text));
    . = 640;
    KEEP(*(.DebugExceptionVector.text));
    . = 704;
    KEEP(*(.NMIExceptionVector.text));
    . = 768;
    KEEP(*(.KernelExceptionVector.text));
    . = 832;
    KEEP(*(.UserExceptionVector.text));
    . = 960;
    KEEP(*(.DoubleExceptionVector.text));
    . = 0x400;

    _init_end = ABSOLUTE(.);
  } > vectors_seg
}

SECTIONS {
  .text : ALIGN(4)
  {
    _stext = .;
    . = ALIGN (4);
    _text_start = ABSOLUTE(.);
    . = ALIGN (4);
    *(.literal .text .literal.* .text.*)
    *(.rwtext.literal .rwtext .rwtext.literal.* .rwtext.*)
    _text_end = ABSOLUTE(.);
    _etext = .;
  } > RWTEXT

  .iram0_reserved_for_text (NOLOAD) : ALIGN(4)
  {
    . = ORIGIN(RWDATA) + SIZEOF(.text);
  } > RWDATA

  .data : ALIGN(4)
  {
    _data_start = ABSOLUTE(.);
    . = ALIGN (4);
    *(.data .data.*)
    *(.rodata .rodata.*)
    _data_end = ABSOLUTE(.);
  } > RWDATA

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