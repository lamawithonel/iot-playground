//! Bounded inter-task message channel abstraction
//!
//! One trait per channel end, matching the common ownership split
//! where a producer task holds the sender and a consumer task holds
//! the receiver.

/// Error from [`MessageSender::try_send`].
///
/// Carries the rejected message so the caller can apply its own
/// drop policy (e.g., drop-newest with a warning log).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum TrySendError<T> {
    /// Channel buffer is at capacity.
    Full(T),
    /// The receive end is closed; delivery is impossible.
    Closed(T),
}

/// Error from [`MessageReceiver::recv`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RecvError {
    /// All senders closed and the buffer is drained.
    Closed,
}

/// Producer end of a bounded inter-task message channel.
pub trait MessageSender {
    /// Message type carried by the channel.
    type Item;

    /// Attempt to enqueue without blocking.
    ///
    /// Returns `Err(TrySendError::Full(item))` when the buffer is at
    /// capacity, returning `item` so a drop-newest policy stays
    /// expressible and countable.
    fn try_send(&mut self, item: Self::Item) -> Result<(), TrySendError<Self::Item>>;
}

/// Consumer end of a bounded inter-task message channel.
pub trait MessageReceiver {
    /// Message type carried by the channel.
    type Item;

    /// Await the next message in FIFO order.
    // Single-core RTIC executor; no `Send` bound wanted (see the
    // design's async-story decision).
    #[allow(async_fn_in_trait)]
    async fn recv(&mut self) -> Result<Self::Item, RecvError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;
    use crate::test_support::{now_or_never, MockChannel, MockRng};

    #[test]
    fn fifo_order_preserved() {
        let channel = MockChannel::<u8, 2>::new();
        let (mut tx, mut rx) = channel.split();
        tx.try_send(1).unwrap();
        tx.try_send(2).unwrap();
        assert_eq!(now_or_never(rx.recv()), Some(Ok(1)));
        assert_eq!(now_or_never(rx.recv()), Some(Ok(2)));
    }

    #[test]
    fn full_returns_rejected_item_and_counts() {
        let channel = MockChannel::<u8, 2>::new();
        let (mut tx, _rx) = channel.split();
        tx.try_send(1).unwrap();
        tx.try_send(2).unwrap();
        assert_eq!(tx.try_send(3), Err(TrySendError::Full(3)));
        assert_eq!(channel.full_events(), 1);
    }

    #[test]
    fn dropped_receiver_closes_sender() {
        let channel = MockChannel::<u8, 2>::new();
        let (mut tx, rx) = channel.split();
        drop(rx);
        assert_eq!(tx.try_send(1), Err(TrySendError::Closed(1)));
    }

    #[test]
    fn dropped_sender_drains_then_closes_receiver() {
        let channel = MockChannel::<u8, 2>::new();
        let (mut tx, mut rx) = channel.split();
        tx.try_send(1).unwrap();
        drop(tx);
        assert_eq!(now_or_never(rx.recv()), Some(Ok(1)));
        assert_eq!(now_or_never(rx.recv()), Some(Err(RecvError::Closed)));
    }

    #[test]
    fn recv_is_pending_on_empty_open_channel() {
        let channel = MockChannel::<u8, 2>::new();
        let (_tx, mut rx) = channel.split();
        assert_eq!(now_or_never(rx.recv()), None);
    }

    /// Bounded, deterministic stand-in for ADR-009's Layer 1 idea
    /// (200 reads with randomized MQTT stalls).  A `MockRng`-driven
    /// schedule assigns each tick a network stall of 0..=6 virtual
    /// ticks; no threads, no wall-clock time, fully rerunnable.
    #[test]
    fn backpressure_conserves_and_orders_readings() {
        const SENSOR_CHANNEL_CAP: usize = 2;
        const TICKS: u32 = 200;

        for seed in 1u64..=4 {
            let channel = MockChannel::<u32, SENSOR_CHANNEL_CAP>::new();
            let (mut tx, mut rx) = channel.split();
            let mut rng = MockRng::seeded(seed);

            let mut delivered: std::vec::Vec<u32> = std::vec::Vec::new();
            let mut accepted = 0u32;
            let mut dropped = 0u32;
            let mut last_accepted: Option<u32> = None;
            let mut stall_remaining: u32 = 0;

            for reading in 0..TICKS {
                match tx.try_send(reading) {
                    Ok(()) => {
                        accepted += 1;
                        last_accepted = Some(reading);
                    }
                    Err(TrySendError::Full(_)) => dropped += 1,
                    Err(TrySendError::Closed(_)) => {
                        panic!("seed {seed}: receiver must not close mid-run")
                    }
                }

                if stall_remaining == 0 {
                    if let Some(Ok(item)) = now_or_never(rx.recv()) {
                        delivered.push(item);
                    }
                    let mut byte = [0u8; 1];
                    rng.fill_bytes(&mut byte);
                    stall_remaining = u32::from(byte[0] % 7); // 0..=6
                } else {
                    stall_remaining -= 1;
                }
            }

            // Drain whatever the stall schedule left buffered.
            while let Some(Ok(item)) = now_or_never(rx.recv()) {
                delivered.push(item);
            }
            // Sender still alive: an empty channel is Pending, not
            // Closed.
            assert_eq!(now_or_never(rx.recv()), None);

            assert_eq!(accepted + dropped, TICKS, "seed {seed}: conservation");
            assert!(
                dropped > 0,
                "seed {seed}: stall schedule must force at least one Full"
            );
            assert_eq!(
                dropped,
                channel.full_events(),
                "seed {seed}: drop count matches full_events"
            );
            assert!(
                delivered.windows(2).all(|w| w[0] < w[1]),
                "seed {seed}: delivered sequence must be strictly increasing"
            );
            assert_eq!(
                delivered.last().copied(),
                last_accepted,
                "seed {seed}: final delivery must be the most recent accepted reading"
            );
        }
    }
}
