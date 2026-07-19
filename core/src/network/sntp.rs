//! SNTP request construction and reply validation (RFC 4330).
//!
//! Pure, platform-agnostic packet handling: build a client request
//! carrying a caller-supplied transmit-timestamp value, and validate
//! a server reply's fixed fields before its time is trusted.  All
//! socket I/O stays in the board crate; these functions only touch
//! byte buffers, so they are host-unit-tested.
//!
//! RFC 4330 section 5 specifies that a unicast client places a value
//! in the request Transmit Timestamp and checks that the reply's
//! Originate Timestamp echoes it, alongside the Mode, Leap Indicator,
//! and Stratum fields.  A reply whose fields do not match the request
//! carries an indeterminate time and must be discarded rather than
//! written to the clock.

/// Reasons a server reply fails validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SntpReplyError {
    /// Fewer than 48 bytes received: not a complete NTP message.
    TooShort,
    /// Reply did not come from the queried server address.
    WrongSource,
    /// Mode field is not 4 (server): not a valid reply to a client.
    NotServerMode,
    /// Leap Indicator is 3 (unsynchronized): the server has no time.
    Unsynchronized,
    /// Stratum is 0 (kiss-o'-death) or above the configured maximum.
    BadStratum,
    /// Originate Timestamp does not echo the request's transmit value.
    OriginateMismatch,
}

/// NTP mode for a server reply.
const MODE_SERVER: u8 = 4;
/// Leap Indicator value meaning the source is not synchronized.
const LI_UNSYNC: u8 = 3;

/// Build a 48-byte NTP client request.
///
/// Byte 0 is `LI=0, VN=3, Mode=3` (client).  The 64-bit `transmit`
/// value is written big-endian into the Transmit Timestamp field
/// (bytes 40..48); the server copies it back into the reply's
/// Originate Timestamp, where [`validate_reply`] checks it.  Callers
/// pass an unpredictable value so a reply can be matched to its
/// request.
pub fn build_request(transmit: u64) -> [u8; 48] {
    let mut pkt = [0u8; 48];
    pkt[0] = 0x1B; // LI=0, VN=3, Mode=3 (client)
    pkt[40..48].copy_from_slice(&transmit.to_be_bytes());
    pkt
}

/// Validate a server reply and extract its Transmit Timestamp.
///
/// `response` is the receive buffer, `recv_len` the number of bytes
/// received, `source_matches` whether the datagram came from the
/// queried server, `sent_transmit` the value [`build_request`] put in
/// the request, and `max_stratum` the highest acceptable stratum.
///
/// On success returns the reply's Transmit Timestamp as
/// `(ntp_seconds, ntp_fraction)` for conversion via
/// `Timestamp::from_ntp`.  On any field mismatch returns the specific
/// [`SntpReplyError`] so the reply is discarded instead of setting the
/// clock to an indeterminate value.
pub fn validate_reply(
    response: &[u8],
    recv_len: usize,
    source_matches: bool,
    sent_transmit: u64,
    max_stratum: u8,
) -> Result<(u64, u32), SntpReplyError> {
    if recv_len < 48 || response.len() < 48 {
        return Err(SntpReplyError::TooShort);
    }
    if !source_matches {
        return Err(SntpReplyError::WrongSource);
    }
    if response[0] & 0x07 != MODE_SERVER {
        return Err(SntpReplyError::NotServerMode);
    }
    if response[0] >> 6 == LI_UNSYNC {
        return Err(SntpReplyError::Unsynchronized);
    }
    let stratum = response[1];
    if stratum == 0 || stratum > max_stratum {
        return Err(SntpReplyError::BadStratum);
    }
    let originate = u64::from_be_bytes([
        response[24],
        response[25],
        response[26],
        response[27],
        response[28],
        response[29],
        response[30],
        response[31],
    ]);
    if originate != sent_transmit {
        return Err(SntpReplyError::OriginateMismatch);
    }
    let secs = u32::from_be_bytes([response[40], response[41], response[42], response[43]]) as u64;
    let frac = u32::from_be_bytes([response[44], response[45], response[46], response[47]]);
    Ok((secs, frac))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONCE: u64 = 0x0123_4567_89AB_CDEF;
    const MAX_STRATUM: u8 = 3;

    fn good_reply() -> [u8; 48] {
        let mut r = [0u8; 48];
        r[0] = 0x1C; // LI=0, VN=3, Mode=4 (server)
        r[1] = 2; // stratum 2
        r[24..32].copy_from_slice(&NONCE.to_be_bytes()); // Originate echoes request
        r[40..44].copy_from_slice(&0x8000_0000u32.to_be_bytes()); // transmit secs
        r[44..48].copy_from_slice(&0x4000_0000u32.to_be_bytes()); // transmit frac
        r
    }

    #[test]
    fn build_request_places_transmit_and_client_mode() {
        let pkt = build_request(NONCE);
        assert_eq!(pkt[0], 0x1B);
        assert_eq!(u64::from_be_bytes(pkt[40..48].try_into().unwrap()), NONCE);
    }

    #[test]
    fn valid_reply_accepted() {
        let r = good_reply();
        let (secs, frac) = validate_reply(&r, 48, true, NONCE, MAX_STRATUM).unwrap();
        assert_eq!(secs, 0x8000_0000);
        assert_eq!(frac, 0x4000_0000);
    }

    #[test]
    fn short_reply_rejected() {
        let r = good_reply();
        assert_eq!(
            validate_reply(&r, 47, true, NONCE, MAX_STRATUM),
            Err(SntpReplyError::TooShort)
        );
    }

    #[test]
    fn wrong_source_rejected() {
        let r = good_reply();
        assert_eq!(
            validate_reply(&r, 48, false, NONCE, MAX_STRATUM),
            Err(SntpReplyError::WrongSource)
        );
    }

    #[test]
    fn non_server_mode_rejected() {
        let mut r = good_reply();
        r[0] = 0x1B; // Mode=3 (client), not a server reply
        assert_eq!(
            validate_reply(&r, 48, true, NONCE, MAX_STRATUM),
            Err(SntpReplyError::NotServerMode)
        );
    }

    #[test]
    fn unsynchronized_reply_rejected() {
        let mut r = good_reply();
        r[0] = 0xDC; // LI=3, Mode=4
        assert_eq!(
            validate_reply(&r, 48, true, NONCE, MAX_STRATUM),
            Err(SntpReplyError::Unsynchronized)
        );
    }

    #[test]
    fn bad_stratum_rejected() {
        let mut r = good_reply();
        r[1] = 0; // kiss-o'-death
        assert_eq!(
            validate_reply(&r, 48, true, NONCE, MAX_STRATUM),
            Err(SntpReplyError::BadStratum)
        );
        r[1] = MAX_STRATUM + 1;
        assert_eq!(
            validate_reply(&r, 48, true, NONCE, MAX_STRATUM),
            Err(SntpReplyError::BadStratum)
        );
    }

    #[test]
    fn originate_mismatch_rejected() {
        let r = good_reply();
        // A reply that does not echo the value we sent carries a time
        // we never requested; it must be discarded.
        assert_eq!(
            validate_reply(&r, 48, true, NONCE ^ 0xFF, MAX_STRATUM),
            Err(SntpReplyError::OriginateMismatch)
        );
    }
}
