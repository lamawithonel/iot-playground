/* STM32N657X0-- Cortex-M55, flashless.  G0 is a RAM-only image; see
   the board page's "Bring-Up: RAM-Boot Dev Flow" section
   (docs/src/boards/nucleo-n657x0.md) for why.  Regions and
   addresses are copied verbatim from embassy's own working
   example, rather than derived fresh:
   https://raw.githubusercontent.com/embassy-rs/embassy/main/examples/stm32n6/memory.x

   Both regions sit inside the AXISRAM123456 *secure* alias
   (0x34000000-0x343c0000, STM32N6_Series.yaml), not the non-secure
   alias (0x24000000)-- the boot ROM's TrustZone state at reset in
   dev-boot mode (BOOT1=1) is presumed to require the secure alias. */
MEMORY
{
  FLASH : ORIGIN = 0x341A0000, LENGTH = 256K
  RAM   : ORIGIN = 0x341E0000, LENGTH = 128K
}

/* No explicit _stack_start override: the workspace uses flip-link
   (root .cargo/config.toml), which places the stack at the bottom
   of RAM for overflow detection. */
