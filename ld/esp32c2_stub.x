INCLUDE memory.x

REGION_ALIAS("ROTEXT", IRAM);
REGION_ALIAS("RODATA", DRAM);

REGION_ALIAS("RWDATA", DRAM);
REGION_ALIAS("RWTEXT", IRAM);

/* include linker script from esp-hal */
INCLUDE esp32c2.x
INCLUDE rom-functions.x
