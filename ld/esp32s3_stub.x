
INCLUDE "memory.x"

/* map generic regions to output sections */
REGION_ALIAS("ROTEXT", iram_seg);
REGION_ALIAS("RWTEXT", iram_seg);
REGION_ALIAS("RODATA", dram_seg);
REGION_ALIAS("RWDATA", dram_seg);

REGION_ALIAS("RTC_FAST_RWTEXT", rtc_fast_seg);
REGION_ALIAS("RTC_FAST_RWDATA", rtc_fast_seg);

/* include linker script from esp-hal */
INCLUDE esp32s3.x
INCLUDE rom-functions.x
INCLUDE hal-defaults.x

