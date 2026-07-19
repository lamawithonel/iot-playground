//! Capture/label record storage sink abstraction
//!
//! `core`'s ARS record types (`CAPTURE RECORD v1` / `LABEL RECORD
//! v1`) serialize themselves to bytes with `to_bytes`; this trait
//! is the missing sink for those bytes.  Concrete sinks vary by
//! rig-- a host-side stream over the debug link for the H7
//! loopback bench, external flash or a network egress on the N6
//! node-- and the capture task must not care which.

/// Append-only sink for serialized ARS records.
///
/// Rationale: record producers (the capture task, the labeler)
/// need exactly one operation-- durably hand off one complete,
/// already-CRC'd record-- so the trait is a single awaitable
/// append.  Framing lives inside the record bytes themselves
/// (magic, declared lengths, CRC trailer), so the sink can
/// concatenate appends and a reader can re-split the stream; no
/// extra framing contract is imposed here.  Reading back is host
/// tooling's job, not the device's, so no read side exists.
pub trait RecordStore {
    /// Implementation-specific storage failure (e.g., flash full,
    /// stream closed).
    type Error: core::fmt::Debug;

    /// Append one complete serialized record.
    ///
    /// `record` is the exact byte image produced by a record's
    /// `to_bytes`-- header through CRC trailer.  Completes once the
    /// sink has accepted the whole record; on error nothing partial
    /// may be retained, so the stream never holds a torn record.
    // Single-core RTIC executor; no `Send` bound wanted (see the
    // design's async-story decision).
    #[allow(async_fn_in_trait)]
    async fn append(&mut self, record: &[u8]) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{now_or_never, MockRecordStore, MockRecordStoreError};

    #[test]
    fn appended_records_are_kept_in_order_and_intact() {
        let mut store = MockRecordStore::<32, 4>::new();
        assert_eq!(now_or_never(store.append(&[0xAA, 0xBB])), Some(Ok(())));
        assert_eq!(now_or_never(store.append(&[0x01])), Some(Ok(())));
        assert_eq!(store.record_count(), 2);
        assert_eq!(store.record(0), Some(&[0xAA, 0xBB][..]));
        assert_eq!(store.record(1), Some(&[0x01][..]));
        assert_eq!(store.record(2), None);
    }

    #[test]
    fn record_slot_exhaustion_is_rejected() {
        let mut store = MockRecordStore::<32, 2>::new();
        assert_eq!(now_or_never(store.append(&[1])), Some(Ok(())));
        assert_eq!(now_or_never(store.append(&[2])), Some(Ok(())));
        assert_eq!(
            now_or_never(store.append(&[3])),
            Some(Err(MockRecordStoreError::Full))
        );
        assert_eq!(store.record_count(), 2);
    }

    #[test]
    fn byte_exhaustion_rejects_whole_record_and_keeps_previous() {
        let mut store = MockRecordStore::<4, 4>::new();
        assert_eq!(now_or_never(store.append(&[9, 8, 7])), Some(Ok(())));
        // 3 + 2 > 4 bytes: rejected whole, no torn record.
        assert_eq!(
            now_or_never(store.append(&[6, 5])),
            Some(Err(MockRecordStoreError::Full))
        );
        assert_eq!(store.record_count(), 1);
        assert_eq!(store.record(0), Some(&[9, 8, 7][..]));
    }

    /// Generic consumer proof: the shape the capture task uses--
    /// serialize into a scratch buffer, then hand the bytes to
    /// whatever store the board wired in.
    #[test]
    fn generic_consumer_bound_is_usable() {
        async fn flush<S: RecordStore>(store: &mut S, bytes: &[u8]) -> Result<(), S::Error> {
            store.append(bytes).await
        }

        let mut store = MockRecordStore::<8, 2>::new();
        assert_eq!(now_or_never(flush(&mut store, &[0x42; 3])), Some(Ok(())));
        assert_eq!(store.record(0), Some(&[0x42; 3][..]));
    }
}
