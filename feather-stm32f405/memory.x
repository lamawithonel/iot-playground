/* STM32F405RG */
/* ROM bootloader is in separate system memory at 0x1FFF0000 */
MEMORY
{
  /* Full 1MB flash starting at 0x08000000 */
  FLASH : ORIGIN = 0x08000000, LENGTH = 1024K
  RAM : ORIGIN = 0x20000000, LENGTH = 128K
}

/* Place stack at end of RAM */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);


/*
   NOTE: There is an additional 64KB of CCM (Core Coupled Memory) RAM
   at address 0x10000000. It is faster for the CPU but CANNOT be used
   for DMA (like USB buffers).

   Standard Rust linker scripts target the 'RAM' region defined above.
   If you need the extra 64KB, you must manually place variables there.
*/
