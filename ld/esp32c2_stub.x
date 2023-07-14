INCLUDE memory.x

REGION_ALIAS("ROTEXT", IRAM);
REGION_ALIAS("RODATA", DRAM);

REGION_ALIAS("RWDATA", DRAM);
REGION_ALIAS("RWTEXT", IRAM);

/* include linker script from esp-hal */
INCLUDE bl-riscv-link.x
INCLUDE rom-functions.x