//! `CAPTURE RECORD v1` ("ARSC") and `LABEL RECORD v1` ("ARSL")
//!
//! Fixed little-endian wire layouts with hand-rolled
//! `to_bytes`/`parse` and a trailing CRC-32 (see [`super::crc32`]).
//! No `serde`, no heap: capture payloads are borrowed slices
//! supplied by the caller, and parsing returns a zero-copy view
//! over the input buffer.
//!
//! Captures are uniquely keyed by `(bench_id, boot_id,
//! capture_seq)`; `mic_id` rides alongside so two rigs' data can
//! never be confused even if files are pooled.  Labels reference
//! captures by that same key rather than embedding them.

use super::crc32;
use super::types::{ExcitationDescriptor, ExcitationKind};

/// Errors returned by capture/label record encoding and decoding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SchemaError {
    /// The output buffer passed to `to_bytes` is too small
    BufferTooSmall,
    /// The magic bytes did not match the expected record type
    BadMagic,
    /// The `version` field is not one this parser understands
    UnsupportedVersion,
    /// The input buffer is shorter than the record it claims to be
    Truncated,
    /// The trailing CRC-32 does not match the computed checksum
    CrcMismatch,
    /// `payload_kind` byte is not a recognized [`PayloadKind`]
    BadPayloadKind,
    /// `exc.kind` byte is not a recognized `ExcitationKind`
    BadExcitationKind,
    /// A label's `hot_end`/`cold_end` byte is not recognized
    BadEndPresence,
    /// A label's `source` byte is not a recognized `LabelSource`
    BadLabelSource,
}

fn read_u16(buf: &[u8], off: usize) -> u16 {
    u16::from_le_bytes(buf[off..off + 2].try_into().expect("slice len checked"))
}

fn read_i16(buf: &[u8], off: usize) -> i16 {
    i16::from_le_bytes(buf[off..off + 2].try_into().expect("slice len checked"))
}

fn read_u32(buf: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(buf[off..off + 4].try_into().expect("slice len checked"))
}

fn read_u64(buf: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(buf[off..off + 8].try_into().expect("slice len checked"))
}

fn write_u16(buf: &mut [u8], off: usize, val: u16) {
    buf[off..off + 2].copy_from_slice(&val.to_le_bytes());
}

fn write_i16(buf: &mut [u8], off: usize, val: i16) {
    buf[off..off + 2].copy_from_slice(&val.to_le_bytes());
}

fn write_u32(buf: &mut [u8], off: usize, val: u32) {
    buf[off..off + 4].copy_from_slice(&val.to_le_bytes());
}

fn write_u64(buf: &mut [u8], off: usize, val: u64) {
    buf[off..off + 8].copy_from_slice(&val.to_le_bytes());
}

/// `CAPTURE RECORD v1` magic bytes
pub const CAPTURE_MAGIC: [u8; 4] = *b"ARSC";
/// `CAPTURE RECORD v1` version number
pub const CAPTURE_VERSION: u16 = 1;
/// `CAPTURE RECORD v1` fixed header length in bytes
pub const CAPTURE_HEADER_LEN: usize = 72;
/// CRC-32 trailer length in bytes (all records)
pub const CRC_LEN: usize = 4;

/// Discriminant for what kind of samples the capture payload holds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PayloadKind {
    /// Raw signed 16-bit Q15 samples
    RawI16 = 0,
    /// Precomputed unsigned 32-bit magnitude bins
    MagnitudeU32 = 1,
}

impl PayloadKind {
    /// Decode from the wire byte value
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::RawI16),
            1 => Some(Self::MagnitudeU32),
            _ => None,
        }
    }

    /// Encode to the wire byte value
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Borrowed capture payload-- either raw samples or magnitude bins
///
/// No heap allocation: the caller owns the backing buffer for the
/// lifetime of the [`CaptureRecord`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Payload<'a> {
    /// Raw Q15 samples
    RawI16(&'a [i16]),
    /// Precomputed magnitude bins
    MagnitudeU32(&'a [u32]),
}

impl<'a> Payload<'a> {
    /// Which [`PayloadKind`] this payload holds
    pub const fn kind(&self) -> PayloadKind {
        match self {
            Self::RawI16(_) => PayloadKind::RawI16,
            Self::MagnitudeU32(_) => PayloadKind::MagnitudeU32,
        }
    }

    /// Number of elements in the payload
    pub const fn count(&self) -> usize {
        match self {
            Self::RawI16(s) => s.len(),
            Self::MagnitudeU32(s) => s.len(),
        }
    }

    fn byte_len(&self) -> usize {
        match self {
            Self::RawI16(s) => s.len() * 2,
            Self::MagnitudeU32(s) => s.len() * 4,
        }
    }
}

/// Fixed capture metadata (everything but the payload)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CaptureHeader {
    /// Rig bench identifier (never `0` on real hardware)
    pub bench_id: u8,
    /// Microphone/sensor unit identifier on that bench
    pub mic_id: u8,
    /// Channel count (`1` for now)
    pub channels: u8,
    /// Random-per-power-up boot identifier
    pub boot_id: u32,
    /// Monotonic-per-boot capture sequence number
    pub capture_seq: u32,
    /// Device monotonic clock at capture start, microseconds
    pub monotonic_us: u64,
    /// Wall-clock time, microseconds since epoch (`0` == unsynced)
    pub unix_micros: u64,
    /// Sample rate, Hz
    pub fs_hz: u32,
    /// What signal was playing during this capture
    pub excitation: ExcitationDescriptor,
}

/// A complete `CAPTURE RECORD v1`, ready to serialize
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureRecord<'a> {
    /// Fixed metadata
    pub header: CaptureHeader,
    /// Sample or magnitude-bin payload
    pub payload: Payload<'a>,
}

impl<'a> CaptureRecord<'a> {
    /// Total encoded length in bytes (header + payload + CRC)
    pub fn encoded_len(&self) -> usize {
        CAPTURE_HEADER_LEN + self.payload.byte_len() + CRC_LEN
    }

    /// Serialize into `out`, returning the number of bytes written
    ///
    /// Fails with [`SchemaError::BufferTooSmall`] if `out` is
    /// shorter than [`CaptureRecord::encoded_len`].
    pub fn to_bytes(&self, out: &mut [u8]) -> Result<usize, SchemaError> {
        let len = self.encoded_len();
        if out.len() < len {
            return Err(SchemaError::BufferTooSmall);
        }
        let buf = &mut out[..len];
        let h = &self.header;
        let exc = &h.excitation;

        buf[0..4].copy_from_slice(&CAPTURE_MAGIC);
        write_u16(buf, 4, CAPTURE_VERSION);
        write_u16(buf, 6, CAPTURE_HEADER_LEN as u16);
        buf[8] = h.bench_id;
        buf[9] = h.mic_id;
        buf[10] = self.payload.kind().as_u8();
        buf[11] = h.channels;
        write_u32(buf, 12, h.boot_id);
        write_u32(buf, 16, h.capture_seq);
        write_u64(buf, 20, h.monotonic_us);
        write_u64(buf, 28, h.unix_micros);
        write_u32(buf, 36, h.fs_hz);
        buf[40] = exc.kind.as_u8();
        buf[41] = exc.flags;
        write_i16(buf, 42, exc.level_q15);
        write_u32(buf, 44, exc.f_start_dhz);
        write_u32(buf, 48, exc.f_stop_dhz);
        write_u16(buf, 52, exc.steps_or_order);
        write_u16(buf, 54, 0); // reserved
        write_u32(buf, 56, exc.dwell);
        write_u32(buf, 60, exc.seed);
        write_u32(buf, 64, self.payload.count() as u32);
        write_u32(buf, 68, 0); // reserved

        let mut off = CAPTURE_HEADER_LEN;
        match &self.payload {
            Payload::RawI16(samples) => {
                for &s in *samples {
                    write_i16(buf, off, s);
                    off += 2;
                }
            }
            Payload::MagnitudeU32(bins) => {
                for &b in *bins {
                    write_u32(buf, off, b);
                    off += 4;
                }
            }
        }

        let crc = crc32::checksum(&buf[..off]);
        write_u32(buf, off, crc);

        Ok(len)
    }
}

/// Zero-copy, validated view over a parsed `CAPTURE RECORD v1`
///
/// [`CaptureView::parse`] validates the magic, version, declared
/// lengths, excitation kind, and CRC once; every accessor below is
/// then an infallible fixed-offset read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureView<'a> {
    buf: &'a [u8],
    header_len: usize,
}

impl<'a> CaptureView<'a> {
    /// Validate and wrap a byte buffer as a `CaptureView`
    ///
    /// `buf` may be longer than the record (trailing bytes are
    /// ignored); it may not be shorter.
    pub fn parse(buf: &'a [u8]) -> Result<Self, SchemaError> {
        if buf.len() < CAPTURE_HEADER_LEN + CRC_LEN {
            return Err(SchemaError::Truncated);
        }
        if buf[0..4] != CAPTURE_MAGIC {
            return Err(SchemaError::BadMagic);
        }
        if read_u16(buf, 4) != CAPTURE_VERSION {
            return Err(SchemaError::UnsupportedVersion);
        }
        let header_len = read_u16(buf, 6) as usize;
        if header_len < CAPTURE_HEADER_LEN {
            return Err(SchemaError::Truncated);
        }
        if buf.len() < header_len {
            return Err(SchemaError::Truncated);
        }
        PayloadKind::from_u8(buf[10]).ok_or(SchemaError::BadPayloadKind)?;
        ExcitationKind::from_u8(buf[40]).ok_or(SchemaError::BadExcitationKind)?;

        let payload_count = read_u32(buf, 64) as usize;
        let elem_size = match PayloadKind::from_u8(buf[10]).expect("checked above") {
            PayloadKind::RawI16 => 2,
            PayloadKind::MagnitudeU32 => 4,
        };
        let payload_bytes = payload_count
            .checked_mul(elem_size)
            .ok_or(SchemaError::Truncated)?;
        let total_len = header_len
            .checked_add(payload_bytes)
            .and_then(|v| v.checked_add(CRC_LEN))
            .ok_or(SchemaError::Truncated)?;
        if buf.len() < total_len {
            return Err(SchemaError::Truncated);
        }

        let record = &buf[..total_len];
        let crc_off = total_len - CRC_LEN;
        let stored_crc = read_u32(record, crc_off);
        let computed_crc = crc32::checksum(&record[..crc_off]);
        if stored_crc != computed_crc {
            return Err(SchemaError::CrcMismatch);
        }

        Ok(Self {
            buf: record,
            header_len,
        })
    }

    /// Rig bench identifier
    pub fn bench_id(&self) -> u8 {
        self.buf[8]
    }

    /// Microphone/sensor unit identifier
    pub fn mic_id(&self) -> u8 {
        self.buf[9]
    }

    /// Which kind of samples the payload holds
    pub fn payload_kind(&self) -> PayloadKind {
        PayloadKind::from_u8(self.buf[10]).expect("validated in parse")
    }

    /// Channel count
    pub fn channels(&self) -> u8 {
        self.buf[11]
    }

    /// Random-per-power-up boot identifier
    pub fn boot_id(&self) -> u32 {
        read_u32(self.buf, 12)
    }

    /// Monotonic-per-boot capture sequence number
    pub fn capture_seq(&self) -> u32 {
        read_u32(self.buf, 16)
    }

    /// Device monotonic clock at capture start, microseconds
    pub fn monotonic_us(&self) -> u64 {
        read_u64(self.buf, 20)
    }

    /// Wall-clock time, microseconds since epoch (`0` == unsynced)
    pub fn unix_micros(&self) -> u64 {
        read_u64(self.buf, 28)
    }

    /// Sample rate, Hz
    pub fn fs_hz(&self) -> u32 {
        read_u32(self.buf, 36)
    }

    /// The excitation descriptor recorded with this capture
    pub fn excitation(&self) -> ExcitationDescriptor {
        ExcitationDescriptor {
            kind: ExcitationKind::from_u8(self.buf[40]).expect("validated in parse"),
            flags: self.buf[41],
            level_q15: read_i16(self.buf, 42),
            f_start_dhz: read_u32(self.buf, 44),
            f_stop_dhz: read_u32(self.buf, 48),
            steps_or_order: read_u16(self.buf, 52),
            dwell: read_u32(self.buf, 56),
            seed: read_u32(self.buf, 60),
        }
    }

    /// Number of payload elements
    pub fn payload_count(&self) -> u32 {
        read_u32(self.buf, 64)
    }

    /// Read payload element `i` as a raw Q15 sample
    ///
    /// Returns `None` if the payload is not [`PayloadKind::RawI16`]
    /// or `i` is out of range.
    pub fn raw_i16_at(&self, i: usize) -> Option<i16> {
        if self.payload_kind() != PayloadKind::RawI16 {
            return None;
        }
        if i as u32 >= self.payload_count() {
            return None;
        }
        Some(read_i16(self.buf, self.header_len + i * 2))
    }

    /// Read payload element `i` as a magnitude bin
    ///
    /// Returns `None` if the payload is not
    /// [`PayloadKind::MagnitudeU32`] or `i` is out of range.
    pub fn magnitude_u32_at(&self, i: usize) -> Option<u32> {
        if self.payload_kind() != PayloadKind::MagnitudeU32 {
            return None;
        }
        if i as u32 >= self.payload_count() {
            return None;
        }
        Some(read_u32(self.buf, self.header_len + i * 4))
    }
}

/// `LABEL RECORD v1` magic bytes
pub const LABEL_MAGIC: [u8; 4] = *b"ARSL";
/// `LABEL RECORD v1` version number
pub const LABEL_VERSION: u16 = 1;
/// `LABEL RECORD v1` fixed record length in bytes
pub const LABEL_RECORD_LEN: usize = 36;

/// Whether an end (hot or cold) was present during a capture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum EndPresence {
    /// The end was absent
    Absent = 0,
    /// The end was present
    Present = 1,
    /// Presence is unknown
    Unknown = 2,
}

impl EndPresence {
    /// Decode from the wire byte value
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Absent),
            1 => Some(Self::Present),
            2 => Some(Self::Unknown),
            _ => None,
        }
    }

    /// Encode to the wire byte value
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Where a label's classification came from
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LabelSource {
    /// Generated by the synthetic plant model
    Synthetic = 0,
    /// Produced by the on-device or host ARS classifier
    ArsClassifier = 1,
    /// Assigned by a human reviewer
    Human = 2,
}

impl LabelSource {
    /// Decode from the wire byte value
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Synthetic),
            1 => Some(Self::ArsClassifier),
            2 => Some(Self::Human),
            _ => None,
        }
    }

    /// Encode to the wire byte value
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

/// A complete `LABEL RECORD v1`
///
/// References its capture by `(bench_id, boot_id, capture_seq)`
/// rather than embedding it, keeping label emission cheap on
/// device and letting host tooling join the two record streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LabelRecord {
    /// Rig bench identifier-- must match the referenced capture
    pub bench_id: u8,
    /// Microphone/sensor unit identifier-- must match the capture
    pub mic_id: u8,
    /// Hot end presence during the referenced capture
    pub hot_end: EndPresence,
    /// Cold end presence during the referenced capture
    pub cold_end: EndPresence,
    /// Boot identifier of the referenced capture
    pub boot_id: u32,
    /// Sequence number of the referenced capture
    pub capture_seq: u32,
    /// Label wall time, microseconds since epoch (`0` == unsynced)
    pub unix_micros: u64,
    /// Confidence in Q15 (`32768` == `1.0`)
    pub confidence_q15: u16,
    /// Where this label came from
    pub source: LabelSource,
}

impl LabelRecord {
    /// Serialize to a fixed 36-byte array
    pub fn to_bytes(&self) -> [u8; LABEL_RECORD_LEN] {
        let mut buf = [0u8; LABEL_RECORD_LEN];
        buf[0..4].copy_from_slice(&LABEL_MAGIC);
        write_u16(&mut buf, 4, LABEL_VERSION);
        write_u16(&mut buf, 6, LABEL_RECORD_LEN as u16);
        buf[8] = self.bench_id;
        buf[9] = self.mic_id;
        buf[10] = self.hot_end.as_u8();
        buf[11] = self.cold_end.as_u8();
        write_u32(&mut buf, 12, self.boot_id);
        write_u32(&mut buf, 16, self.capture_seq);
        write_u64(&mut buf, 20, self.unix_micros);
        write_u16(&mut buf, 28, self.confidence_q15);
        buf[30] = self.source.as_u8();
        buf[31] = 0; // reserved
        let crc = crc32::checksum(&buf[..32]);
        write_u32(&mut buf, 32, crc);
        buf
    }

    /// Validate and decode a 36-byte (or longer, trailing-ignored)
    /// buffer
    pub fn parse(buf: &[u8]) -> Result<Self, SchemaError> {
        if buf.len() < LABEL_RECORD_LEN {
            return Err(SchemaError::Truncated);
        }
        if buf[0..4] != LABEL_MAGIC {
            return Err(SchemaError::BadMagic);
        }
        if read_u16(buf, 4) != LABEL_VERSION {
            return Err(SchemaError::UnsupportedVersion);
        }
        if (read_u16(buf, 6) as usize) < LABEL_RECORD_LEN {
            return Err(SchemaError::Truncated);
        }
        let stored_crc = read_u32(buf, 32);
        let computed_crc = crc32::checksum(&buf[..32]);
        if stored_crc != computed_crc {
            return Err(SchemaError::CrcMismatch);
        }
        Ok(Self {
            bench_id: buf[8],
            mic_id: buf[9],
            hot_end: EndPresence::from_u8(buf[10]).ok_or(SchemaError::BadEndPresence)?,
            cold_end: EndPresence::from_u8(buf[11]).ok_or(SchemaError::BadEndPresence)?,
            boot_id: read_u32(buf, 12),
            capture_seq: read_u32(buf, 16),
            unix_micros: read_u64(buf, 20),
            confidence_q15: read_u16(buf, 28),
            source: LabelSource::from_u8(buf[30]).ok_or(SchemaError::BadLabelSource)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn golden_capture_header() -> CaptureHeader {
        CaptureHeader {
            bench_id: 3,
            mic_id: 1,
            channels: 1,
            boot_id: 0xDEAD_BEEF,
            capture_seq: 42,
            monotonic_us: 123_456_789,
            unix_micros: 0,
            fs_hz: 48_000,
            excitation: ExcitationDescriptor {
                kind: ExcitationKind::Sine,
                flags: 0,
                level_q15: 16_000,
                f_start_dhz: 12_000,
                f_stop_dhz: 12_000,
                steps_or_order: 0,
                dwell: 4,
                seed: 0,
            },
        }
    }

    /// Byte-exact golden vector cross-checked against an
    /// independent Python `struct.pack` + `zlib.crc32` encoding of
    /// the same field values (see the design's synthetic generator
    /// plan for the golden-corpus rationale).
    #[rustfmt::skip]
    const GOLDEN_CAPTURE_BYTES: [u8; 84] = [
        0x41, 0x52, 0x53, 0x43, 0x01, 0x00, 0x48, 0x00, 0x03, 0x01, 0x00, 0x01,
        0xEF, 0xBE, 0xAD, 0xDE, 0x2A, 0x00, 0x00, 0x00, 0x15, 0xCD, 0x5B, 0x07,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x80, 0xBB, 0x00, 0x00, 0x00, 0x00, 0x80, 0x3E, 0xE0, 0x2E, 0x00, 0x00,
        0xE0, 0x2E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x64, 0x00, 0x38, 0xFF, 0x2C, 0x01, 0x70, 0xFE, 0x5A, 0xE5, 0x6F, 0x32,
    ];

    #[test]
    fn test_capture_to_bytes_matches_golden_vector() {
        let payload_samples: [i16; 4] = [100, -200, 300, -400];
        let record = CaptureRecord {
            header: golden_capture_header(),
            payload: Payload::RawI16(&payload_samples),
        };
        let mut out = [0u8; 84];
        let n = record.to_bytes(&mut out).unwrap();
        assert_eq!(n, 84);
        assert_eq!(out, GOLDEN_CAPTURE_BYTES);
    }

    #[test]
    fn test_capture_golden_vector_roundtrips_through_parse() {
        let view = CaptureView::parse(&GOLDEN_CAPTURE_BYTES).unwrap();
        assert_eq!(view.bench_id(), 3);
        assert_eq!(view.mic_id(), 1);
        assert_eq!(view.channels(), 1);
        assert_eq!(view.payload_kind(), PayloadKind::RawI16);
        assert_eq!(view.boot_id(), 0xDEAD_BEEF);
        assert_eq!(view.capture_seq(), 42);
        assert_eq!(view.monotonic_us(), 123_456_789);
        assert_eq!(view.unix_micros(), 0);
        assert_eq!(view.fs_hz(), 48_000);
        assert_eq!(view.payload_count(), 4);
        assert_eq!(view.raw_i16_at(0), Some(100));
        assert_eq!(view.raw_i16_at(1), Some(-200));
        assert_eq!(view.raw_i16_at(2), Some(300));
        assert_eq!(view.raw_i16_at(3), Some(-400));
        assert_eq!(view.raw_i16_at(4), None);
        assert_eq!(view.magnitude_u32_at(0), None);

        let exc = view.excitation();
        assert_eq!(exc.kind, ExcitationKind::Sine);
        assert_eq!(exc.level_q15, 16_000);
        assert_eq!(exc.f_start_dhz, 12_000);
        assert_eq!(exc.f_stop_dhz, 12_000);
        assert_eq!(exc.dwell, 4);
    }

    #[test]
    fn test_capture_buffer_too_small_is_rejected() {
        let payload_samples: [i16; 4] = [1, 2, 3, 4];
        let record = CaptureRecord {
            header: golden_capture_header(),
            payload: Payload::RawI16(&payload_samples),
        };
        let mut out = [0u8; 10];
        assert_eq!(record.to_bytes(&mut out), Err(SchemaError::BufferTooSmall));
    }

    #[test]
    fn test_capture_parse_rejects_bad_magic() {
        let mut bytes = GOLDEN_CAPTURE_BYTES;
        bytes[0] = b'X';
        assert_eq!(CaptureView::parse(&bytes), Err(SchemaError::BadMagic));
    }

    #[test]
    fn test_capture_parse_rejects_crc_mismatch() {
        let mut bytes = GOLDEN_CAPTURE_BYTES;
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;
        assert_eq!(CaptureView::parse(&bytes), Err(SchemaError::CrcMismatch));
    }

    #[test]
    fn test_capture_parse_rejects_truncated_buffer() {
        let bytes = &GOLDEN_CAPTURE_BYTES[..40];
        assert_eq!(CaptureView::parse(bytes), Err(SchemaError::Truncated));
    }

    #[test]
    fn test_capture_magnitude_payload_roundtrip() {
        let bins: [u32; 3] = [10, 2_000_000_000, 42];
        let record = CaptureRecord {
            header: golden_capture_header(),
            payload: Payload::MagnitudeU32(&bins),
        };
        let mut out = [0u8; 128];
        let n = record.to_bytes(&mut out).unwrap();
        let view = CaptureView::parse(&out[..n]).unwrap();
        assert_eq!(view.payload_kind(), PayloadKind::MagnitudeU32);
        assert_eq!(view.payload_count(), 3);
        assert_eq!(view.magnitude_u32_at(0), Some(10));
        assert_eq!(view.magnitude_u32_at(1), Some(2_000_000_000));
        assert_eq!(view.magnitude_u32_at(2), Some(42));
        assert_eq!(view.raw_i16_at(0), None);
    }

    fn golden_label() -> LabelRecord {
        LabelRecord {
            bench_id: 3,
            mic_id: 1,
            hot_end: EndPresence::Present,
            cold_end: EndPresence::Absent,
            boot_id: 0xDEAD_BEEF,
            capture_seq: 42,
            unix_micros: 1_700_000_000_000_000,
            confidence_q15: 32_768,
            source: LabelSource::ArsClassifier,
        }
    }

    /// Byte-exact golden vector, cross-checked the same way as
    /// [`GOLDEN_CAPTURE_BYTES`].
    #[rustfmt::skip]
    const GOLDEN_LABEL_BYTES: [u8; 36] = [
        0x41, 0x52, 0x53, 0x4C, 0x01, 0x00, 0x24, 0x00, 0x03, 0x01, 0x01, 0x00,
        0xEF, 0xBE, 0xAD, 0xDE, 0x2A, 0x00, 0x00, 0x00, 0x00, 0x40, 0x1E, 0x18,
        0x24, 0x0A, 0x06, 0x00, 0x00, 0x80, 0x01, 0x00, 0xBD, 0x6B, 0x5B, 0xEC,
    ];

    #[test]
    fn test_label_to_bytes_matches_golden_vector() {
        assert_eq!(golden_label().to_bytes(), GOLDEN_LABEL_BYTES);
    }

    #[test]
    fn test_label_golden_vector_roundtrips_through_parse() {
        let label = LabelRecord::parse(&GOLDEN_LABEL_BYTES).unwrap();
        assert_eq!(label, golden_label());
    }

    #[test]
    fn test_label_parse_rejects_crc_mismatch() {
        let mut bytes = GOLDEN_LABEL_BYTES;
        bytes[35] ^= 0xFF;
        assert_eq!(LabelRecord::parse(&bytes), Err(SchemaError::CrcMismatch));
    }

    #[test]
    fn test_label_parse_rejects_truncated_buffer() {
        assert_eq!(
            LabelRecord::parse(&GOLDEN_LABEL_BYTES[..20]),
            Err(SchemaError::Truncated)
        );
    }
}
