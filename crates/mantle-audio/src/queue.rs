use std::fmt;

use rtrb::{Consumer, Producer, PushError, RingBuffer};

use super::EncodedFrameSlot;

/// Lavaplayer's default five-second buffer at one 20 ms frame, plus its current-frame slot.
pub const DEFAULT_ENCODED_FRAME_QUEUE_CAPACITY: usize = 251;

/// Current per-track queue limit: five seconds plus one compatible output frame.
pub const MAX_ENCODED_FRAME_QUEUE_CAPACITY: usize = DEFAULT_ENCODED_FRAME_QUEUE_CAPACITY;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodedFrameQueueConfigError {
    ZeroCapacity,
    CapacityExceeded { requested: usize, limit: usize },
}

impl fmt::Display for EncodedFrameQueueConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroCapacity => {
                formatter.write_str("encoded frame queue capacity must be non-zero")
            }
            Self::CapacityExceeded { requested, limit } => write!(
                formatter,
                "encoded frame queue capacity {requested} exceeds the per-track limit of {limit}"
            ),
        }
    }
}

impl std::error::Error for EncodedFrameQueueConfigError {}

/// A failed non-blocking write that retains ownership of the undelivered frame.
#[derive(Debug)]
pub struct EncodedFrameQueueFull(EncodedFrameSlot);

impl EncodedFrameQueueFull {
    #[must_use]
    pub fn into_frame(self) -> EncodedFrameSlot {
        self.0
    }
}

impl fmt::Display for EncodedFrameQueueFull {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("encoded frame queue is full")
    }
}

impl std::error::Error for EncodedFrameQueueFull {}

/// The single producer side of a bounded encoded-frame queue.
#[derive(Debug)]
pub struct EncodedFrameProducer {
    inner: Producer<EncodedFrameSlot>,
}

impl EncodedFrameProducer {
    /// Attempts to transfer a frame without waiting or allocating.
    ///
    /// A full queue returns ownership of `frame` in the error.
    ///
    /// # Errors
    ///
    /// Returns `EncodedFrameQueueFull` when no slot is currently available. Its inline frame is
    /// intentionally not boxed because recovering backpressure must remain allocation-free.
    #[allow(
        clippy::result_large_err,
        reason = "boxing the returned inline frame would allocate on backpressure"
    )]
    pub fn try_push(&mut self, frame: EncodedFrameSlot) -> Result<(), EncodedFrameQueueFull> {
        self.inner.push(frame).map_err(|error| match error {
            PushError::Full(frame) => EncodedFrameQueueFull(frame),
        })
    }

    #[must_use]
    pub fn remaining_capacity(&self) -> usize {
        self.inner.slots()
    }

    #[must_use]
    pub fn is_full(&self) -> bool {
        self.inner.is_full()
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.inner.buffer().capacity()
    }
}

/// The single consumer side of a bounded encoded-frame queue.
#[derive(Debug)]
pub struct EncodedFrameConsumer {
    inner: Consumer<EncodedFrameSlot>,
}

impl EncodedFrameConsumer {
    /// Moves the next frame into `target` without waiting or allocating.
    #[must_use]
    pub fn try_pop_into(&mut self, target: &mut EncodedFrameSlot) -> bool {
        let Ok(frame) = self.inner.pop() else {
            return false;
        };
        *target = frame;
        true
    }

    /// Drops all currently visible frames, for example after a seek or track reset.
    ///
    /// A producer on another thread may publish a new frame while this method runs. The caller
    /// must coordinate pipeline generation changes when an atomic discontinuity is required.
    pub fn clear(&mut self) -> usize {
        let mut cleared = 0_usize;
        while self.inner.pop().is_ok() {
            cleared = cleared.saturating_add(1);
        }
        cleared
    }

    #[must_use]
    pub fn available(&self) -> usize {
        self.inner.slots()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.inner.buffer().capacity()
    }
}

/// Allocates one fixed-capacity ring, then returns its independent SPSC endpoints.
///
/// # Errors
///
/// Returns an error for zero capacity or a request beyond the per-track resource limit.
pub fn encoded_frame_queue(
    capacity: usize,
) -> Result<(EncodedFrameProducer, EncodedFrameConsumer), EncodedFrameQueueConfigError> {
    if capacity == 0 {
        return Err(EncodedFrameQueueConfigError::ZeroCapacity);
    }
    if capacity > MAX_ENCODED_FRAME_QUEUE_CAPACITY {
        return Err(EncodedFrameQueueConfigError::CapacityExceeded {
            requested: capacity,
            limit: MAX_ENCODED_FRAME_QUEUE_CAPACITY,
        });
    }
    let (producer, consumer) = RingBuffer::new(capacity);
    Ok((
        EncodedFrameProducer { inner: producer },
        EncodedFrameConsumer { inner: consumer },
    ))
}

#[cfg(test)]
mod tests {
    use std::mem;
    use std::thread;
    use std::time::Duration;

    use super::{
        EncodedFrameQueueConfigError, MAX_ENCODED_FRAME_QUEUE_CAPACITY, encoded_frame_queue,
    };
    use crate::{EncodedFrameSlot, VolumeLevel};

    fn frame(sequence: usize) -> EncodedFrameSlot {
        let mut frame = EncodedFrameSlot::new();
        frame
            .write(
                &sequence.to_le_bytes(),
                Some(Duration::from_millis(u64::try_from(sequence).unwrap() * 20)),
                VolumeLevel::NORMAL,
            )
            .unwrap();
        frame
    }

    #[test]
    fn configuration_is_hard_bounded() {
        assert_eq!(
            encoded_frame_queue(0).unwrap_err(),
            EncodedFrameQueueConfigError::ZeroCapacity
        );
        assert_eq!(
            encoded_frame_queue(MAX_ENCODED_FRAME_QUEUE_CAPACITY + 1).unwrap_err(),
            EncodedFrameQueueConfigError::CapacityExceeded {
                requested: MAX_ENCODED_FRAME_QUEUE_CAPACITY + 1,
                limit: MAX_ENCODED_FRAME_QUEUE_CAPACITY,
            }
        );
    }

    #[test]
    fn full_empty_fifo_and_clear_contracts_are_explicit() {
        let (mut producer, mut consumer) = encoded_frame_queue(2).unwrap();
        assert_eq!(producer.capacity(), 2);
        assert_eq!(consumer.capacity(), 2);
        assert!(consumer.is_empty());

        producer.try_push(frame(1)).unwrap();
        producer.try_push(frame(2)).unwrap();
        assert!(producer.is_full());
        let rejected = producer.try_push(frame(3)).unwrap_err().into_frame();
        assert_eq!(rejected.data(), 3_usize.to_le_bytes());

        let mut output = EncodedFrameSlot::new();
        assert!(consumer.try_pop_into(&mut output));
        assert_eq!(output.data(), 1_usize.to_le_bytes());
        assert_eq!(consumer.clear(), 1);
        assert!(!consumer.try_pop_into(&mut output));
        assert_eq!(output.data(), 1_usize.to_le_bytes());

        producer.try_push(mem::take(&mut output)).unwrap();
        assert!(consumer.try_pop_into(&mut output));
        assert_eq!(output.data(), 1_usize.to_le_bytes());
    }

    #[test]
    fn endpoints_transfer_fifty_thousand_frames_in_order_across_threads() {
        const FRAMES: usize = 50_000;
        let (mut producer, mut consumer) = encoded_frame_queue(17).unwrap();

        thread::scope(|scope| {
            scope.spawn(move || {
                for sequence in 0..FRAMES {
                    let mut pending = frame(sequence);
                    loop {
                        match producer.try_push(pending) {
                            Ok(()) => break,
                            Err(full) => {
                                pending = full.into_frame();
                                thread::yield_now();
                            }
                        }
                    }
                }
            });

            let mut output = EncodedFrameSlot::new();
            for expected in 0..FRAMES {
                while !consumer.try_pop_into(&mut output) {
                    thread::yield_now();
                }
                assert_eq!(output.data(), expected.to_le_bytes());
                assert_eq!(
                    output.timestamp(),
                    Some(Duration::from_millis(u64::try_from(expected).unwrap() * 20))
                );
            }
        });
    }
}
