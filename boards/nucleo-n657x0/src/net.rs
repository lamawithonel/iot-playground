//! Networking helpers for the on-chip Ethernet bring-up (`net`
//! feature).
//!
//! Compiled only under the additive `net` feature (which requires
//! `g1-spike`).  Holds the two board-specific values embassy-net
//! needs that phase-1 has no home for yet: a link-layer MAC address
//! and the stack's ISN seed.  Both are derived from the STM32N657's
//! factory-programmed 96-bit unique ID (`embassy_stm32::uid`) so
//! each board is stable and distinct on the wire without a
//! hardcoded, collision-prone constant, and with no `unsafe`.
//!
//! This deliberately avoids the hardware RNG: DHCP and ICMP need no
//! cryptographic entropy, so a UID-derived seed keeps the bring-up
//! free of an extra peripheral claim and interrupt binding.  A
//! future TLS/MQTT phase (see `docs/src/boards/nucleo-n657x0.md`)
//! is the point to introduce `embassy_stm32::rng` for a real
//! entropy source.

#![deny(unsafe_code)]
#![deny(warnings)]

/// Build a stable, locally-administered unicast MAC from the UID.
///
/// The first octet is forced to a locally-administered unicast
/// value (bit 1 set, bit 0 clear) per IEEE 802 so it can never
/// collide with a real vendor OUI; the remaining five octets carry
/// UID entropy.
pub fn mac_address() -> [u8; 6] {
    let uid = embassy_stm32::uid::uid();
    let mut mac = [0u8; 6];
    mac.copy_from_slice(&uid[0..6]);
    // Locally administered (0x02), unicast (clear 0x01).
    mac[0] = (mac[0] | 0x02) & 0xFE;
    mac
}

/// Derive the embassy-net stack seed (TCP/UDP ISN randomization)
/// from the UID.
///
/// Uses the UID's high eight bytes so it does not simply echo the
/// MAC's source bytes.  Not a cryptographic seed-- adequate for
/// transaction-ID diversity on the bench, not for TLS.
pub fn stack_seed() -> u64 {
    let uid = embassy_stm32::uid::uid();
    let mut seed = [0u8; 8];
    seed.copy_from_slice(&uid[4..12]);
    u64::from_le_bytes(seed)
}
