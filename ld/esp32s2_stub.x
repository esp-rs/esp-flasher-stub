RESERVE_CACHES = 32k;
VECTORS_SIZE   = 0x400;

ENTRY(ESP32Reset)

/* Specify main memory areas */
MEMORY
{
  vectors_seg ( RX ) : ORIGIN = 0x40020000 + RESERVE_CACHES, len = VECTORS_SIZE /* SRAM0 */
  iram_seg    ( RX ) : ORIGIN = 0x40020000 + RESERVE_CACHES + VECTORS_SIZE, len = 192k - RESERVE_CACHES - VECTORS_SIZE /* SRAM0 */
  dram_seg    ( RW ) : ORIGIN = 0x3FFB0000 + 0x4000 + RESERVE_CACHES + VECTORS_SIZE, len = 188k - RESERVE_CACHES - VECTORS_SIZE

  /* RTC fast memory (executable). Persists over deep sleep. Only for core 0 (PRO_CPU) */
  rtc_fast_iram_seg (RWX) : ORIGIN = 0x40070000, len = 8k

  /* RTC fast memory (same block as above), viewed from data bus. Only for core 0 (PRO_CPU) */
  rtc_fast_dram_seg (RW) : ORIGIN = 0x3ff9e000, len = 8k

  /* RTC slow memory (data accessible). Persists over deep sleep. */
  rtc_slow_seg (RW) : ORIGIN = 0x50000000, len = 8k
}

/* map generic regions to output sections */
REGION_ALIAS("ROTEXT", iram_seg);
REGION_ALIAS("RWTEXT", iram_seg);
REGION_ALIAS("RODATA", dram_seg);
REGION_ALIAS("RWDATA", dram_seg);

REGION_ALIAS("RTC_FAST_RWTEXT", rtc_fast_iram_seg);
REGION_ALIAS("RTC_FAST_RWDATA", rtc_fast_dram_seg);

/* include linker script from esp-hal */
INCLUDE esp32s2.x
INCLUDE rom-functions.x
INCLUDE hal-defaults.x
