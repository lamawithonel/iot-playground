//! Cryptographically secure random byte source

/// Cryptographically secure random byte source.
///
/// Contract: implementations MUST draw from a hardware TRNG or a
/// CSPRNG seeded from one-- output may feed TLS session key
/// material.  `fill_bytes` is infallible; implementations handle
/// entropy stalls internally (block until ready), matching
/// `rand_core::RngCore::fill_bytes` semantics exactly so any
/// `RngCore` implementor delegates in one line.
///
/// ```rust,ignore
/// fn client_id_salt<R: Rng>(rng: &mut R) -> [u8; 4] {
///     let mut salt = [0u8; 4];
///     rng.fill_bytes(&mut salt);
///     salt
/// }
/// ```
pub trait Rng {
    /// Fill `dest` completely with random bytes.
    fn fill_bytes(&mut self, dest: &mut [u8]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::MockRng;

    #[test]
    fn same_seed_same_stream() {
        let mut a = MockRng::seeded(42);
        let mut b = MockRng::seeded(42);
        let mut buf_a = [0u8; 64];
        let mut buf_b = [0u8; 64];
        a.fill_bytes(&mut buf_a);
        b.fill_bytes(&mut buf_b);
        assert_eq!(buf_a, buf_b);
    }

    #[test]
    fn different_seed_different_stream() {
        let mut a = MockRng::seeded(42);
        let mut b = MockRng::seeded(43);
        let mut buf_a = [0u8; 64];
        let mut buf_b = [0u8; 64];
        a.fill_bytes(&mut buf_a);
        b.fill_bytes(&mut buf_b);
        assert_ne!(buf_a, buf_b);
    }

    #[test]
    fn chunked_fill_matches_single_fill() {
        let mut chunked = MockRng::seeded(7);
        let mut whole = MockRng::seeded(7);

        let mut c = [0u8; 41];
        let (first, rest) = c.split_at_mut(1);
        chunked.fill_bytes(first);
        let (second, third) = rest.split_at_mut(7);
        chunked.fill_bytes(second);
        chunked.fill_bytes(third);

        let mut w = [0u8; 41];
        whole.fill_bytes(&mut w);

        assert_eq!(c, w);
    }

    #[test]
    fn zero_length_fill_is_noop() {
        let mut with_noop_call = MockRng::seeded(5);
        with_noop_call.fill_bytes(&mut []);
        let mut buf_with_noop_call = [0u8; 8];
        with_noop_call.fill_bytes(&mut buf_with_noop_call);

        let mut without_noop_call = MockRng::seeded(5);
        let mut buf_without_noop_call = [0u8; 8];
        without_noop_call.fill_bytes(&mut buf_without_noop_call);

        assert_eq!(buf_with_noop_call, buf_without_noop_call);
    }

    /// Generic consumer proof: this is the shape a board TLS-provider
    /// adapter would use to consume the `Rng` bound.
    fn nonce<R: Rng>(rng: &mut R) -> [u8; 12] {
        let mut out = [0u8; 12];
        rng.fill_bytes(&mut out);
        out
    }

    #[test]
    fn generic_consumer_bound_is_usable() {
        let mut via_generic = MockRng::seeded(99);
        let n = nonce(&mut via_generic);

        let mut direct = MockRng::seeded(99);
        let mut expected = [0u8; 12];
        direct.fill_bytes(&mut expected);

        assert_eq!(n, expected);
    }
}
