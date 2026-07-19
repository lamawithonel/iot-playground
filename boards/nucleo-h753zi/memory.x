/* STM32H753ZI -- DS12117 Rev 10 Sec 3.3, RM0433 Rev 8 Table 7 */
MEMORY
{
  /* 2 MB flash, two contiguous 1 MB banks:
     0x08000000-0x080FFFFF (bank 1) + 0x08100000-0x081FFFFF (bank 2)
     (DS12117 Rev 10 p.25 Sec 3.3.1; RM0433 Rev 8 p.130-131 Table 7) */
  FLASH : ORIGIN = 0x08000000, LENGTH = 2048K

  /* DTCM-RAM: 128 KB, 0-wait-state, 0x20000000-0x2001FFFF
     (DS12117 Rev 10 p.26 Sec 3.3.3; RM0433 Rev 8 p.131 Table 7)
     Holds .data/.bss and the stack for phase 1. */
  RAM : ORIGIN = 0x20000000, LENGTH = 128K

  /* Documented for later phases, no sections yet:
     AXISRAM 512K @ 0x24000000 (D1) -- future DAC/ADC DMA buffers
     SRAM1 128K @ 0x30000000, SRAM2 128K @ 0x30020000,
     SRAM3 32K @ 0x30040000 (D2) -- future Ethernet DMA
     descriptors/buffers, SRAM4 64K @ 0x38000000 (D3)
     (DS12117 Rev 10 p.26; RM0433 Rev 8 p.131 Table 7) */
}

/* No explicit _stack_start override: the workspace uses flip-link
   (root .cargo/config.toml), which relocates the stack below
   .data/.bss for overflow detection, and RAM already starts at
   the cortex-m-rt-conventional 0x20000000 origin. */
