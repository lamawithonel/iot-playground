//! CRC-32/IEEE checksum
//!
//! Hand-rolled table-based implementation (no external `crc`
//! crate, per the crate's serde-free/minimal-dependency policy).
//! The 256-entry table is built at compile time via `const fn`, so
//! there is no runtime setup cost and no `unsafe` is needed.

const fn build_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut j = 0;
        while j < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            j += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
}

const TABLE: [u32; 256] = build_table();

/// Compute the CRC-32/IEEE checksum of `data`
///
/// This is the same polynomial and reflection used by `zlib`,
/// `gzip`, and Ethernet FCS-- host tooling (e.g. Python's
/// `zlib.crc32`) can verify records byte-for-byte.
pub fn checksum(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        let idx = ((crc ^ byte as u32) & 0xFF) as usize;
        crc = TABLE[idx] ^ (crc >> 8);
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        // Well-known CRC-32/IEEE value for the empty string.
        assert_eq!(checksum(&[]), 0x0000_0000);
    }

    #[test]
    fn test_known_vector_check() {
        // The canonical CRC-32 check value for the ASCII string
        // "123456789" (used by every CRC catalog, e.g.
        // reveng.sourceforge.io's "CRC-32/ISO-HDLC" entry).
        assert_eq!(checksum(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn test_deterministic() {
        let data = b"ARSC deterministic golden vector input";
        assert_eq!(checksum(data), checksum(data));
    }

    #[test]
    fn test_single_bit_change_alters_checksum() {
        let a = checksum(b"ARSC0");
        let b = checksum(b"ARSC1");
        assert_ne!(a, b);
    }
}
