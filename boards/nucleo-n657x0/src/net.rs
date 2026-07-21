//! Networking helpers for the on-chip Ethernet + MQTT bring-up
//! (`net` feature).
//!
//! Compiled only under the additive `net` feature (which requires
//! `g1-spike`).  Holds the board-specific values the shared network
//! stack and clients need but that have no other home: a link-layer
//! MAC address, the embassy-net stack seed, and the MQTT client
//! identifier.  All three derive from the STM32N657's
//! factory-programmed 96-bit unique ID (`embassy_stm32::uid`) so each
//! board is stable and distinct on the wire and on the broker without
//! a hardcoded, collision-prone constant, and with no `unsafe`.
//!
//! These UID-derived values are exactly the ones that must stay
//! *stable per board*, where a per-boot random value would be wrong.
//! Cryptographic entropy-- TLS 1.3 handshake randomness and the SNTP
//! transmit nonce-- comes instead from the hardware RNG
//! (`embassy_stm32::rng`), initialized in `main.rs`'s `network_task`.

#![deny(unsafe_code)]
#![deny(warnings)]

use heapless::String;

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
/// transaction-ID diversity on the bench.  TLS entropy is a separate
/// concern handled by the hardware RNG (see the module docs).
pub fn stack_seed() -> u64 {
    let uid = embassy_stm32::uid::uid();
    let mut seed = [0u8; 8];
    seed.copy_from_slice(&uid[4..12]);
    u64::from_le_bytes(seed)
}

/// Maximum MQTT client-ID length: `"n657-"` (5) + 24 UID hex chars.
const CLIENT_ID_MAX_LEN: usize = 29;

/// Derive a stable MQTT client ID from the factory UID.
///
/// Format: `n657-{24 hex chars}`.  Stable across reboots and unique
/// per chip, so two nodes never clash on the broker.  The resulting
/// telemetry topic (`device/n657-{uid}/telemetry`, 46 chars) stays
/// within `iot_core::network::mqtt::MAX_TOPIC_LEN` (64).  Mirrors
/// feather's `device_id::mqtt_client_id` (`stm32f405-{uid}`).
pub fn mqtt_client_id() -> String<CLIENT_ID_MAX_LEN> {
    // Both pushes fit CLIENT_ID_MAX_LEN exactly (5 + 24), so neither
    // can fail; mirrors feather's `.expect` documentation of the same
    // fixed-capacity invariant.
    let mut id = String::new();
    id.push_str("n657-").expect("prefix fits");
    id.push_str(embassy_stm32::uid::uid_hex())
        .expect("uid hex fits");
    id
}
