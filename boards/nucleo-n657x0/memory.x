/* STM32N657X0-- Cortex-M55, flashless.  G0 is a RAM-only image; see
   the board page's "Bring-Up: RAM-Boot Dev Flow" section
   (docs/src/boards/nucleo-n657x0.md) for why.

   Both regions sit inside the AXISRAM123456 *secure* alias
   (0x34000000-0x343c0000, STM32N6_Series.yaml), not the non-secure
   alias (0x24000000)-- the boot ROM's TrustZone state at reset in
   dev-boot mode (BOOT1=1) is presumed to require the secure alias.

   Bank map (from embassy's own N6 example memory.x header):

     FLEXRAM  0x34000000   400 KB
     AXISRAM1 0x34064000   624 KB   (needs RCC.memenr.axisram1en = 1; off at reset)
     AXISRAM2 0x34100000  1024 KB   (enabled by the boot ROM -- always safe)
     AXISRAM3 0x34200000   448 KB   (reset-enable state not assumed here)
     ...

   Both FLASH and RAM live entirely inside AXISRAM2, the one bank the
   boot ROM guarantees enabled at reset, so no RCC bank-enable code
   (which the flashless embassy example avoids too) is needed.

   FLASH ORIGIN is pinned at 0x341A0000: the RAM-boot loader of record
   sets the initial SP/PC from the vector table at that fixed address
   (see the board page's bring-up section), and cortex-m-rt places the
   vector table at FLASH ORIGIN.  Moving it would break the load path.

   Sizing: the earlier DHCPv4-only image fit a 256K/128K carve at the
   top of AXISRAM2.  The MQTT-over-TLS path (net feature) needs ~34 KB
   of TLS 1.3 record buffers plus TCP/MQTT buffers, the embassy-net
   packet ring, and a deep TLS-handshake stack, none of which fit
   128K.  This crate forbids `unsafe`, so the feather trick of parking
   the TLS buffers in a separate CCM-RAM region is unavailable-- the
   buffers live in ordinary RAM here.  With FLASH pinned to the top
   384K of AXISRAM2 (0x341A0000-0x34200000, ample for the ~347K image),
   RAM takes the whole lower 640K of the bank (0x34100000-0x341A0000),
   which embassy's example documents as free app RAM.  RAM sits below
   FLASH numerically-- the linker does not require otherwise-- and
   flip-link puts the stack at the RAM top (0x341A0000), i.e. exactly
   the loader's SP, growing down. */
MEMORY
{
  RAM   : ORIGIN = 0x34100000, LENGTH = 640K
  FLASH : ORIGIN = 0x341A0000, LENGTH = 384K
}

/* No explicit _stack_start override: the workspace uses flip-link
   (root .cargo/config.toml), which places the stack at the bottom
   of RAM for overflow detection. */
