/* STM32F405RG Memory Configuration */
/* Total SRAM: 192KB (128KB main + 64KB CCM) */

MEMORY
{
  /* 1MB Flash starting at 0x08000000 */
  FLASH : ORIGIN = 0x08000000, LENGTH = 1024K

  /* Main SRAM: 128KB at 0x20000000 */
  /* Used for: stack, TCP/MQTT buffers, W5500 DMA, sensor data */
  RAM : ORIGIN = 0x20000000, LENGTH = 128K

  /* CCM (Core Coupled Memory): 64KB at 0x10000000 */
  /* CPU-only access, zero wait states, NO DMA */
  /* Used for: TLS buffers, critical data (via .ccmram section) */
  CCMRAM : ORIGIN = 0x10000000, LENGTH = 64K
}

/* Place stack at top of main SRAM (grows downward) */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);

/* Define CCM section for explicitly placed variables */
SECTIONS
{
  .ccmram (NOLOAD) : ALIGN(4)
  {
    *(.ccmram .ccmram.*);
    . = ALIGN(4);
  } > CCMRAM
}

/*
 Memory Usage Strategy (no heap: this firmware does not link
 `alloc`; all buffers are static or stack).  CCM RAM allocations
 are defined in `src/ccmram.rs`, which is the source of truth.

 Main SRAM (128KB) - DMA-accessible:
 ├─ Stack (at top, grows down)
 ├─ TCP RX/TX buffers (StaticCell, ~8KB)
 ├─ MQTT buffer (StaticCell, 2KB)
 ├─ W5500 DMA buffers
 └─ Sensor data buffers

 CCM RAM (64KB) - CPU-only, zero wait states, NO DMA:
 ├─ TLS buffers:              34KB
 │   ├─ TLS_READ_BUF:  18KB
 │   └─ TLS_WRITE_BUF: 16KB
 ├─ Critical variables:      <1KB
 │   └─ TIME_SYNCED + wall-clock base atomics
 └─ Reserved for future:     ~30KB

 Note: the TLS read/write buffers live in CCM (not main SRAM) to
       free SRAM for the deep embedded-tls handshake stack; the
       stack itself stays in main RAM to avoid a linker conflict
       with the .ccmram section.
*/
