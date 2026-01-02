/* STM32F405RG Memory Configuration */
/* Total SRAM: 192KB (128KB main + 64KB CCM) */

MEMORY
{
  /* 1MB Flash starting at 0x08000000 */
  FLASH : ORIGIN = 0x08000000, LENGTH = 1024K

  /* Main SRAM: 128KB at 0x20000000 */
  /* Used for: stack, network buffers, TLS state, heap, application data */
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
 Memory Usage Strategy:

 Main SRAM (128KB) - DMA-accessible:
 ├─ Stack:                    16KB (at top, grows down)
 ├─ TLS session state:        40KB
 ├─ TCP/IP buffers:           20KB
 ├─ Application heap:         20KB
 ├─ W5500 DMA buffers:        12KB
 ├─ Sensor data buffers:       8KB
 ├─ Protobuf encoding:         8KB
 └─ Firmware update buffer:    4KB

 CCM RAM (64KB) - CPU-only, zero wait states:
 ├─ TLS read buffer:          16KB (.ccmram section)
 ├─ TLS write buffer:          8KB (.ccmram section)
 ├─ MQTT buffers:             16KB (.ccmram section)
 └─ Critical variables:       24KB (.ccmram section)

 Note: Stack in main RAM allows more flexibility and prevents
       linker conflicts between stack and .ccmram section.
*/
