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

use crate::message_port::{MessageReceiver, MessageSender, RecvError, TrySendError};
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
