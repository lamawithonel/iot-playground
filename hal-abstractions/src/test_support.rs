//! Host-test doubles for hal-abstractions traits
//!
//! Enabled for this crate's own tests (`cfg(test)`) and for
//! downstream crates via the `mock` feature (typically as a
//! dev-dependency).  Stays `no_std`-clean: a downstream crate that
//! enables `mock` without itself being under `cfg(test)` still
//! compiles this crate as `no_std` (see the crate's
//! `#![cfg_attr(not(test), no_std)]`), so no `std`/`alloc` types are
//! used here.

use core::cell::RefCell;
use core::future::{poll_fn, Future};
use core::pin::pin;
use core::task::{Context, Poll, Waker};

use crate::adc_capture::{AdcCapture, TriggeredCapture};
use crate::excitation::ExcitationSink;
use crate::i2c_bus::I2cBus;
use crate::message_port::{MessageReceiver, MessageSender, RecvError, TrySendError};
use crate::mute_control::MuteControl;
use crate::network::NetworkReadiness;
use crate::record_store::RecordStore;
use crate::rng::Rng;
use crate::rtc::Rtc;
use crate::time::{RtcError, Timestamp};

/// Poll a future exactly once with a no-op waker.
///
/// Returns `None` if the future is still pending.  A
/// dependency-free stand-in for `futures::FutureExt::now_or_never`,
/// used to drive [`MockChannel`] one simulated tick at a time
/// without threads or real wakers.
pub fn now_or_never<F: Future>(fut: F) -> Option<F::Output> {
    let mut fut = pin!(fut);
    let mut cx = Context::from_waker(Waker::noop());
    match fut.as_mut().poll(&mut cx) {
        Poll::Ready(value) => Some(value),
        Poll::Pending => None,
    }
}

/// In-memory [`Rtc`] test double.
///
/// Starts unsynchronized; [`MockRtc::set`] both stores the time and
/// marks the clock synced, mirroring the feather RTC glue.
pub struct MockRtc {
    time: Option<Timestamp>,
    synced: bool,
    pending_failure: Option<RtcError>,
}

impl MockRtc {
    /// Create a fresh, unsynchronized mock clock.
    pub fn new() -> Self {
        Self {
            time: None,
            synced: false,
            pending_failure: None,
        }
    }

    /// Advance the stored clock by `secs` seconds.
    ///
    /// No-op if the clock has never been set.
    pub fn advance_secs(&mut self, secs: u64) {
        if let Some(t) = &mut self.time {
            *t = Timestamp::new(t.unix_secs + secs, t.micros);
        }
    }

    /// Make exactly the next `now`/`set` call fail with `err`, then
    /// recover to normal behavior.
    pub fn fail_next(&mut self, err: RtcError) {
        self.pending_failure = Some(err);
    }
}

impl Default for MockRtc {
    fn default() -> Self {
        Self::new()
    }
}

impl Rtc for MockRtc {
    fn now(&mut self) -> Result<Timestamp, RtcError> {
        if let Some(err) = self.pending_failure.take() {
            return Err(err);
        }
        self.time.ok_or(RtcError::NotInitialized)
    }

    fn set(&mut self, timestamp: Timestamp) -> Result<(), RtcError> {
        if let Some(err) = self.pending_failure.take() {
            return Err(err);
        }
        self.time = Some(timestamp);
        self.synced = true;
        Ok(())
    }

    fn is_synced(&self) -> bool {
        self.synced
    }
}

/// Deterministic xorshift64 [`Rng`] test double.
///
/// Two mocks seeded identically produce byte-for-byte identical
/// streams regardless of how callers chunk their `fill_bytes`
/// calls-- the state is a continuous byte cursor, not a per-call
/// draw, so `MockRng` has real stream semantics to test against.
pub struct MockRng {
    state: u64,
    pending: [u8; 8],
    pending_pos: usize,
}

impl MockRng {
    /// Create a mock seeded with `seed`.
    pub fn seeded(seed: u64) -> Self {
        Self {
            // Avalanche the seed (splitmix64) before use so nearby
            // seeds (e.g., 42 and 43) diverge immediately-- a bare
            // `seed | 1` would leave adjacent even/odd seeds only
            // one bit apart.  xorshift64 also requires a non-zero
            // state, which the `| 1` on the mixed value guarantees.
            state: Self::splitmix64(seed) | 1,
            pending: [0; 8],
            pending_pos: 8,
        }
    }

    fn splitmix64(seed: u64) -> u64 {
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
}

impl Rng for MockRng {
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        let mut i = 0;
        while i < dest.len() {
            if self.pending_pos == 8 {
                self.pending = self.next_u64().to_le_bytes();
                self.pending_pos = 0;
            }
            let avail = 8 - self.pending_pos;
            let need = dest.len() - i;
            let take = avail.min(need);
            dest[i..i + take]
                .copy_from_slice(&self.pending[self.pending_pos..self.pending_pos + take]);
            self.pending_pos += take;
            i += take;
        }
    }
}

/// Fixed-capacity ring buffer backing [`MockChannel`].
struct Ring<T, const N: usize> {
    slots: [Option<T>; N],
    head: usize,
    len: usize,
}

impl<T, const N: usize> Ring<T, N> {
    fn new() -> Self {
        Self {
            slots: core::array::from_fn(|_| None),
            head: 0,
            len: 0,
        }
    }

    fn is_full(&self) -> bool {
        self.len == N
    }

    fn push(&mut self, item: T) {
        debug_assert!(!self.is_full());
        let idx = (self.head + self.len) % N;
        self.slots[idx] = Some(item);
        self.len += 1;
    }

    fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        let item = self.slots[self.head].take();
        self.head = (self.head + 1) % N;
        self.len -= 1;
        item
    }
}

/// Shared state behind a [`MockChannel`], borrowed by both ends
/// through a `RefCell` (single-threaded by design; see the crate's
/// mock design decisions).
struct ChannelState<T, const N: usize> {
    ring: Ring<T, N>,
    sender_alive: bool,
    receiver_alive: bool,
    full_events: u32,
}

/// In-memory [`MessageSender`]/[`MessageReceiver`] test double.
///
/// A fixed-capacity ring buffer over a `RefCell`, matched to the
/// single-core, single-threaded execution model the traits target.
pub struct MockChannel<T, const N: usize> {
    state: RefCell<ChannelState<T, N>>,
}

impl<T, const N: usize> MockChannel<T, N> {
    /// Create a new, empty channel with both ends alive.
    pub fn new() -> Self {
        Self {
            state: RefCell::new(ChannelState {
                ring: Ring::new(),
                sender_alive: true,
                receiver_alive: true,
                full_events: 0,
            }),
        }
    }

    /// Split into a sender and receiver borrowing this channel.
    pub fn split(&self) -> (MockSender<'_, T, N>, MockReceiver<'_, T, N>) {
        (MockSender { channel: self }, MockReceiver { channel: self })
    }

    /// Number of `try_send` calls rejected because the buffer was
    /// full.
    pub fn full_events(&self) -> u32 {
        self.state.borrow().full_events
    }
}

impl<T, const N: usize> Default for MockChannel<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

/// Producer handle returned by [`MockChannel::split`].
pub struct MockSender<'a, T, const N: usize> {
    channel: &'a MockChannel<T, N>,
}

impl<T, const N: usize> MessageSender for MockSender<'_, T, N> {
    type Item = T;

    fn try_send(&mut self, item: T) -> Result<(), TrySendError<T>> {
        let mut state = self.channel.state.borrow_mut();
        if !state.receiver_alive {
            return Err(TrySendError::Closed(item));
        }
        if state.ring.is_full() {
            state.full_events += 1;
            return Err(TrySendError::Full(item));
        }
        state.ring.push(item);
        Ok(())
    }
}

impl<T, const N: usize> Drop for MockSender<'_, T, N> {
    fn drop(&mut self) {
        self.channel.state.borrow_mut().sender_alive = false;
    }
}

/// Consumer handle returned by [`MockChannel::split`].
pub struct MockReceiver<'a, T, const N: usize> {
    channel: &'a MockChannel<T, N>,
}

impl<T, const N: usize> MessageReceiver for MockReceiver<'_, T, N> {
    type Item = T;

    async fn recv(&mut self) -> Result<T, RecvError> {
        poll_fn(|_cx| {
            let mut state = self.channel.state.borrow_mut();
            if let Some(item) = state.ring.pop() {
                Poll::Ready(Ok(item))
            } else if !state.sender_alive {
                Poll::Ready(Err(RecvError::Closed))
            } else {
                Poll::Pending
            }
        })
        .await
    }
}

impl<T, const N: usize> Drop for MockReceiver<'_, T, N> {
    fn drop(&mut self) {
        self.channel.state.borrow_mut().receiver_alive = false;
    }
}

/// Error type for [`MockAdcCapture`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockAdcCaptureError {
    /// The requested window exceeds `MAX_WINDOW_LEN`.
    WindowTooLong,
    /// Injected via [`MockAdcCapture::fail_next`], standing in for
    /// a hardware overrun.
    Overrun,
}

/// Deterministic [`AdcCapture`] test double.
///
/// Replays a fixed sample pattern cyclically with real stream
/// semantics: consecutive windows continue where the previous one
/// ended, regardless of how callers chunk their capture calls
/// (mirroring [`MockRng`]'s byte-cursor design).
pub struct MockAdcCapture<'a> {
    pattern: &'a [i16],
    cursor: usize,
    sample_rate_hz: u32,
    windows_captured: u32,
    pending_failure: Option<MockAdcCaptureError>,
}

impl<'a> MockAdcCapture<'a> {
    /// Create a mock replaying `pattern` at `sample_rate_hz`.
    ///
    /// # Panics
    ///
    /// Panics if `pattern` is empty-- an empty pattern has no
    /// samples to replay.
    pub fn new(pattern: &'a [i16], sample_rate_hz: u32) -> Self {
        assert!(!pattern.is_empty(), "pattern must not be empty");
        Self {
            pattern,
            cursor: 0,
            sample_rate_hz,
            windows_captured: 0,
            pending_failure: None,
        }
    }

    /// Number of windows successfully captured so far.
    pub fn windows_captured(&self) -> u32 {
        self.windows_captured
    }

    /// Make exactly the next `capture` call fail with `err`, then
    /// recover to normal behavior without consuming pattern samples.
    pub fn fail_next(&mut self, err: MockAdcCaptureError) {
        self.pending_failure = Some(err);
    }
}

impl AdcCapture for MockAdcCapture<'_> {
    type Error = MockAdcCaptureError;

    const MAX_WINDOW_LEN: usize = 4096;

    fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    async fn capture(&mut self, window: &mut [i16]) -> Result<(), Self::Error> {
        if let Some(err) = self.pending_failure.take() {
            return Err(err);
        }
        if window.len() > Self::MAX_WINDOW_LEN {
            return Err(MockAdcCaptureError::WindowTooLong);
        }
        for slot in window.iter_mut() {
            *slot = self.pattern[self.cursor];
            self.cursor = (self.cursor + 1) % self.pattern.len();
        }
        self.windows_captured += 1;
        Ok(())
    }
}

/// Error type for [`MockExcitation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockExcitationError {
    /// The block would overflow the mock's fixed recording
    /// capacity `N`; nothing from the block is retained.
    CapacityExceeded,
}

/// Recording [`ExcitationSink`] test double.
///
/// Appends every written block into a fixed `[i16; N]` so tests
/// can assert exactly what a sweep engine emitted, in order.
/// Rejects (whole) blocks that would overflow `N`.
pub struct MockExcitation<const N: usize> {
    emitted: [i16; N],
    len: usize,
}

impl<const N: usize> MockExcitation<N> {
    /// Create an empty recording sink.
    pub fn new() -> Self {
        Self {
            emitted: [0; N],
            len: 0,
        }
    }

    /// All samples written so far, in emission order.
    pub fn emitted(&self) -> &[i16] {
        &self.emitted[..self.len]
    }
}

impl<const N: usize> Default for MockExcitation<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> ExcitationSink for MockExcitation<N> {
    type Error = MockExcitationError;

    async fn write(&mut self, samples: &[i16]) -> Result<(), Self::Error> {
        if self.len + samples.len() > N {
            return Err(MockExcitationError::CapacityExceeded);
        }
        self.emitted[self.len..self.len + samples.len()].copy_from_slice(samples);
        self.len += samples.len();
        Ok(())
    }
}

/// Error type for [`MockRecordStore`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockRecordStoreError {
    /// Byte arena or record-slot capacity exhausted; the rejected
    /// record is not retained, even partially.
    Full,
}

/// Collecting [`RecordStore`] test double.
///
/// Stores appended records contiguously in a fixed `[u8; N]` byte
/// arena with up to `M` per-record lengths, so tests can read each
/// record back intact and in order.  No heap, matching the crate's
/// `no_std`-clean mock rule.
pub struct MockRecordStore<const N: usize, const M: usize> {
    bytes: [u8; N],
    used: usize,
    lens: [usize; M],
    count: usize,
}

impl<const N: usize, const M: usize> MockRecordStore<N, M> {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            bytes: [0; N],
            used: 0,
            lens: [0; M],
            count: 0,
        }
    }

    /// Number of records appended so far.
    pub fn record_count(&self) -> usize {
        self.count
    }

    /// The bytes of record `i` (append order), or `None` if `i` is
    /// out of range.
    pub fn record(&self, i: usize) -> Option<&[u8]> {
        if i >= self.count {
            return None;
        }
        let start: usize = self.lens[..i].iter().sum();
        Some(&self.bytes[start..start + self.lens[i]])
    }
}

impl<const N: usize, const M: usize> Default for MockRecordStore<N, M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize, const M: usize> RecordStore for MockRecordStore<N, M> {
    type Error = MockRecordStoreError;

    async fn append(&mut self, record: &[u8]) -> Result<(), Self::Error> {
        if self.count == M || self.used + record.len() > N {
            return Err(MockRecordStoreError::Full);
        }
        self.bytes[self.used..self.used + record.len()].copy_from_slice(record);
        self.lens[self.count] = record.len();
        self.used += record.len();
        self.count += 1;
        Ok(())
    }
}

/// Error type for [`MockI2cBus`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockI2cBusError {
    /// Recording capacity `M` exhausted, or a single write exceeded
    /// the per-write byte capacity `L`; nothing from the write is
    /// retained.
    Full,
}

/// Recording [`I2cBus`] test double.
///
/// Stores up to `M` writes, each up to `L` bytes, in a fixed
/// `[(u8, usize, [u8; L]); M]` array-- no heap, matching the
/// crate's `no_std`-clean mock rule-- so tests can assert exactly
/// which `(address, bytes)` pairs a driver issued, in order.
pub struct MockI2cBus<const M: usize, const L: usize> {
    writes: [(u8, usize, [u8; L]); M],
    count: usize,
}

impl<const M: usize, const L: usize> MockI2cBus<M, L> {
    /// Create an empty recording bus.
    pub fn new() -> Self {
        Self {
            writes: core::array::from_fn(|_| (0u8, 0usize, [0u8; L])),
            count: 0,
        }
    }

    /// Number of writes recorded so far.
    pub fn write_count(&self) -> usize {
        self.count
    }

    /// The `(address, bytes)` pair for write `i` (issue order), or
    /// `None` if `i` is out of range.
    pub fn write(&self, i: usize) -> Option<(u8, &[u8])> {
        if i >= self.count {
            return None;
        }
        let (address, len, data) = &self.writes[i];
        Some((*address, &data[..*len]))
    }
}

impl<const M: usize, const L: usize> Default for MockI2cBus<M, L> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const M: usize, const L: usize> I2cBus for MockI2cBus<M, L> {
    type Error = MockI2cBusError;

    async fn write(&mut self, address: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        if self.count == M || bytes.len() > L {
            return Err(MockI2cBusError::Full);
        }
        let mut data = [0u8; L];
        data[..bytes.len()].copy_from_slice(bytes);
        self.writes[self.count] = (address, bytes.len(), data);
        self.count += 1;
        Ok(())
    }
}

/// Error type for [`MockMuteControl`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockMuteControlError {
    /// Injected via [`MockMuteControl::fail_next`], standing in for
    /// a GPIO write fault on the amp's mute line.
    GpioFault,
}

/// In-memory [`MuteControl`] test double.
///
/// Tracks both the semantic mute state and the physical line level
/// a real GPIO-backed implementation would drive, mirroring the
/// Adafruit amp board's inverted MUTE_INV sense (drive the header
/// pin LOW to engage mute-- the opposite of the bare MAX9744
/// datasheet; see
/// `docs/src/projects/ars-toolhead-sensor/pinout.md`).  Starts
/// unmuted with the line high, matching the board's default-high
/// pull-up (R15).
pub struct MockMuteControl {
    muted: bool,
    line_driven_low: bool,
    pending_failure: Option<MockMuteControlError>,
}

impl MockMuteControl {
    /// Create a fresh, unmuted mock.
    pub fn new() -> Self {
        Self {
            muted: false,
            line_driven_low: false,
            pending_failure: None,
        }
    }

    /// Whether the simulated physical line is currently driven LOW.
    ///
    /// Test-only inspection hook (not part of [`MuteControl`])
    /// standing in for a logic-analyzer probe on the real GPIO pin--
    /// this is what actually proves the documented inversion, since
    /// [`MuteControl::is_muted`] alone would pass even with the
    /// polarity backwards.
    pub fn line_driven_low(&self) -> bool {
        self.line_driven_low
    }

    /// Make exactly the next `mute`/`unmute` call fail with `err`,
    /// then recover to normal behavior.
    ///
    /// The failed call leaves both the semantic mute state and the
    /// simulated line level unchanged, matching a GPIO write that
    /// never reached the pin.
    pub fn fail_next(&mut self, err: MockMuteControlError) {
        self.pending_failure = Some(err);
    }
}

impl Default for MockMuteControl {
    fn default() -> Self {
        Self::new()
    }
}

impl MuteControl for MockMuteControl {
    type Error = MockMuteControlError;

    fn mute(&mut self) -> Result<(), Self::Error> {
        if let Some(err) = self.pending_failure.take() {
            return Err(err);
        }
        self.muted = true;
        // Adafruit board inversion: engaging mute means driving the
        // header pin LOW (pinout.md, AMP_MUTE_N).
        self.line_driven_low = true;
        Ok(())
    }

    fn unmute(&mut self) -> Result<(), Self::Error> {
        if let Some(err) = self.pending_failure.take() {
            return Err(err);
        }
        self.muted = false;
        self.line_driven_low = false;
        Ok(())
    }

    fn is_muted(&self) -> bool {
        self.muted
    }
}

/// Deterministic [`TriggeredCapture`] test double.
///
/// Extends [`MockAdcCapture`]'s pattern replay with a fixed,
/// caller-declared trigger-to-first-sample latency, standing in for
/// a real TIM-trigger/ADC synchronization measurement (see
/// `docs/src/projects/ars-toolhead-sensor/hil-measurements.md`).
pub struct MockTriggeredCapture<'a> {
    inner: MockAdcCapture<'a>,
    trigger_latency_samples: u32,
}

impl<'a> MockTriggeredCapture<'a> {
    /// Create a mock replaying `pattern` at `sample_rate_hz`,
    /// reporting a fixed `trigger_latency_samples` on every
    /// [`TriggeredCapture::capture_after_trigger`] call.
    pub fn new(pattern: &'a [i16], sample_rate_hz: u32, trigger_latency_samples: u32) -> Self {
        Self {
            inner: MockAdcCapture::new(pattern, sample_rate_hz),
            trigger_latency_samples,
        }
    }

    /// Make exactly the next `capture`/`capture_after_trigger` call
    /// fail with `err`, then recover to normal behavior without
    /// consuming pattern samples.
    ///
    /// Passes through to the wrapped [`MockAdcCapture`], so it
    /// stands in for the same hardware faults (e.g., a DMA overrun)
    /// [`MockAdcCapture::fail_next`] documents.
    pub fn fail_next(&mut self, err: MockAdcCaptureError) {
        self.inner.fail_next(err);
    }
}

impl AdcCapture for MockTriggeredCapture<'_> {
    type Error = MockAdcCaptureError;

    // Mirrors `MockAdcCapture::MAX_WINDOW_LEN`; kept as a literal
    // since an associated const cannot forward a lifetime-generic
    // type's const in a `const` context.
    const MAX_WINDOW_LEN: usize = 4096;

    fn sample_rate_hz(&self) -> u32 {
        self.inner.sample_rate_hz()
    }

    async fn capture(&mut self, window: &mut [i16]) -> Result<(), Self::Error> {
        self.inner.capture(window).await
    }
}

impl TriggeredCapture for MockTriggeredCapture<'_> {
    async fn capture_after_trigger(&mut self, window: &mut [i16]) -> Result<u32, Self::Error> {
        self.inner.capture(window).await?;
        Ok(self.trigger_latency_samples)
    }
}

/// In-memory [`NetworkReadiness`] test double.
///
/// Starts link-down and unconfigured, matching a board at power-up
/// before its link driver reports a PHY link and `iot-net`'s DHCP
/// wait completes.  Link and config state are set independently so
/// tests can exercise the "link up, DHCP pending" case that a bare
/// `embassy_net::Stack::wait_config_up` cannot distinguish from "no
/// link at all".
pub struct MockNetworkReadiness {
    link_up: bool,
    config_up: bool,
}

impl MockNetworkReadiness {
    /// Create a fresh mock: link down, unconfigured.
    pub fn new() -> Self {
        Self {
            link_up: false,
            config_up: false,
        }
    }

    /// Set the simulated physical link state.
    pub fn set_link_up(&mut self, up: bool) {
        self.link_up = up;
    }

    /// Set the simulated IP configuration state.
    ///
    /// A `false` -> `true` transition is what resolves a pending
    /// [`NetworkReadiness::wait_config_up`] call.
    pub fn set_config_up(&mut self, up: bool) {
        self.config_up = up;
    }
}

impl Default for MockNetworkReadiness {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkReadiness for MockNetworkReadiness {
    fn is_link_up(&self) -> bool {
        self.link_up
    }

    async fn wait_config_up(&mut self) {
        let config_up = &self.config_up;
        poll_fn(|_cx| {
            if *config_up {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await
    }
}
