//! Portable bounded channel for `no_std` environments.
//!
//! A simple SPSC-style (but multi-sender safe) channel built on `critical-section`
//! and `heapless::Deque`. Thread/interrupt safe via critical sections.

use core::cell::RefCell;

use critical_section::Mutex;
use heapless::Deque;

/// Error returned when trying to send to a full channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrySendError<T>(pub T);

/// Error returned when trying to receive from an empty channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TryReceiveError;

/// A bounded, thread-safe channel.
///
/// This channel uses critical sections for synchronization, making it suitable
/// for embedded environments. The channel is backed by a fixed-size
/// `heapless::Deque`.
pub struct Channel<T, const SIZE: usize> {
    inner: Mutex<RefCell<Deque<T, SIZE>>>,
}

impl<T, const SIZE: usize> Channel<T, SIZE> {
    /// Create a new empty channel.
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(RefCell::new(Deque::new())),
        }
    }

    /// Get a sender handle for this channel.
    ///
    /// Multiple senders can coexist; they share access to the same queue.
    pub const fn sender(&self) -> Sender<'_, T, SIZE> {
        Sender { channel: self }
    }

    /// Get a receiver handle for this channel.
    ///
    /// Typically only one receiver should drain the queue, but multiple
    /// receivers are allowed (they will compete for messages).
    pub const fn receiver(&self) -> Receiver<'_, T, SIZE> {
        Receiver { channel: self }
    }

    /// Try to send a value into the channel.
    ///
    /// Returns `Err(TrySendError(value))` if the channel is full.
    pub fn try_send(&self, value: T) -> Result<(), TrySendError<T>> {
        critical_section::with(|cs| {
            let mut queue = self.inner.borrow(cs).borrow_mut();
            queue.push_back(value).map_err(TrySendError)
        })
    }

    /// Try to receive a value from the channel.
    ///
    /// Returns `Err(TryReceiveError)` if the channel is empty.
    pub fn try_receive(&self) -> Result<T, TryReceiveError> {
        critical_section::with(|cs| {
            let mut queue = self.inner.borrow(cs).borrow_mut();
            queue.pop_front().ok_or(TryReceiveError)
        })
    }
}

impl<T, const SIZE: usize> Default for Channel<T, SIZE> {
    fn default() -> Self {
        Self::new()
    }
}

/// A sender handle for a [`Channel`].
///
/// This is a lightweight reference that can be cloned and passed around.
#[derive(Clone, Copy)]
pub struct Sender<'a, T, const SIZE: usize> {
    channel: &'a Channel<T, SIZE>,
}

impl<T, const SIZE: usize> Sender<'_, T, SIZE> {
    /// Try to send a value into the channel.
    ///
    /// Returns `Err(TrySendError(value))` if the channel is full.
    pub fn try_send(&self, value: T) -> Result<(), TrySendError<T>> {
        self.channel.try_send(value)
    }
}

/// A receiver handle for a [`Channel`].
///
/// This is a lightweight reference that can be cloned and passed around.
#[derive(Clone, Copy)]
pub struct Receiver<'a, T, const SIZE: usize> {
    channel: &'a Channel<T, SIZE>,
}

impl<T, const SIZE: usize> Receiver<'_, T, SIZE> {
    /// Try to receive a value from the channel.
    ///
    /// Returns `Err(TryReceiveError)` if the channel is empty.
    pub fn try_receive(&self) -> Result<T, TryReceiveError> {
        self.channel.try_receive()
    }
}
