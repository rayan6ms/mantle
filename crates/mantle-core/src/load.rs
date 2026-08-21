use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt;
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LoadId(u64);

impl LoadId {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LoadKey(String);

impl LoadKey {
    /// Creates a non-empty key within the supplied byte bound.
    ///
    /// # Errors
    ///
    /// Returns [`LoadError::InvalidKey`] for an empty or oversized key.
    pub fn new(value: impl Into<String>, maximum_bytes: usize) -> Result<Self, LoadError> {
        let value = value.into();
        if value.is_empty() || value.len() > maximum_bytes {
            return Err(LoadError::InvalidKey);
        }
        Ok(Self(value))
    }
}

/// Non-zero ordering token for FFI and other identity/equality adapters.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OpaqueLoadKey(u64);

impl OpaqueLoadKey {
    /// Creates a key from a boundary-owned stable token.
    ///
    /// # Errors
    ///
    /// Returns [`LoadError::InvalidKey`] for the reserved zero token.
    pub const fn from_opaque(value: u64) -> Result<Self, LoadError> {
        if value == 0 {
            Err(LoadError::InvalidKey)
        } else {
            Ok(Self(value))
        }
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoadState {
    Pending,
    Running,
    Cancelled,
    Complete,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduledLoad<T, K = LoadKey> {
    pub id: LoadId,
    pub key: Option<K>,
    pub value: T,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CancelledLoad<T> {
    pub previous_state: LoadState,
    pub value: Option<T>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoadError {
    InvalidKey,
    QueueFull,
    TooManyChannels,
    ChannelFull,
    UnknownLoad,
    NotRunning,
    Shutdown,
}

impl fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidKey => "load ordering key is invalid or exceeds its limit",
            Self::QueueFull => "load queue is full",
            Self::TooManyChannels => "ordered load channel limit reached",
            Self::ChannelFull => "ordered load channel is full",
            Self::UnknownLoad => "unknown load",
            Self::NotRunning => "load is not running",
            Self::Shutdown => "load scheduler is shut down",
        })
    }
}

impl std::error::Error for LoadError {}

#[derive(Clone, Debug)]
struct Entry<T, K> {
    key: Option<K>,
    value: Option<T>,
    state: LoadState,
}

/// Bounded deterministic scheduling primitive. Equal keys run in submission order.
#[derive(Clone, Debug)]
pub struct LoadScheduler<T, K = LoadKey> {
    maximum_pending: usize,
    maximum_channels: usize,
    maximum_per_channel: usize,
    next_id: u64,
    entries: BTreeMap<LoadId, Entry<T, K>>,
    terminal: VecDeque<(LoadId, LoadState)>,
    ready: VecDeque<LoadId>,
    channels: HashMap<K, VecDeque<LoadId>>,
    shutdown: bool,
}

impl<T> LoadScheduler<T> {
    /// Creates a scheduler using validated text ordering keys.
    #[must_use]
    pub fn new(
        maximum_pending: usize,
        maximum_channels: usize,
        maximum_per_channel: usize,
    ) -> Self {
        Self::new_for_keys(maximum_pending, maximum_channels, maximum_per_channel)
    }
}

impl<T, K> LoadScheduler<T, K>
where
    K: Clone + Eq + Hash,
{
    /// Creates a scheduler for a caller-defined equality/ordering key type.
    #[must_use]
    pub fn new_for_keys(
        maximum_pending: usize,
        maximum_channels: usize,
        maximum_per_channel: usize,
    ) -> Self {
        Self {
            maximum_pending,
            maximum_channels,
            maximum_per_channel,
            next_id: 1,
            entries: BTreeMap::new(),
            terminal: VecDeque::new(),
            ready: VecDeque::new(),
            channels: HashMap::new(),
            shutdown: false,
        }
    }

    /// Adds bounded work to an unordered or ordered channel.
    ///
    /// # Errors
    ///
    /// Returns a resource error when a bound is reached, or [`LoadError::Shutdown`].
    pub fn submit(&mut self, key: Option<K>, value: T) -> Result<LoadId, LoadError> {
        self.submit_recover(key, value).map_err(|(error, _)| error)
    }

    /// Adds bounded work while returning ownership of rejected work to the caller.
    ///
    /// # Errors
    ///
    /// Returns the resource/lifecycle error together with the unconsumed value.
    pub fn submit_recover(&mut self, key: Option<K>, value: T) -> Result<LoadId, (LoadError, T)> {
        if self.shutdown {
            return Err((LoadError::Shutdown, value));
        }
        if self.pending_count() >= self.maximum_pending {
            return Err((LoadError::QueueFull, value));
        }
        if let Some(key) = &key {
            let is_new = !self.channels.contains_key(key);
            if is_new && self.channels.len() >= self.maximum_channels {
                return Err((LoadError::TooManyChannels, value));
            }
            if self.channels.get(key).map_or(0, VecDeque::len) >= self.maximum_per_channel {
                return Err((LoadError::ChannelFull, value));
            }
        }

        let id = LoadId(self.next_id);
        self.next_id = self.next_id.checked_add(1).unwrap_or(1);
        self.entries.insert(
            id,
            Entry {
                key: key.clone(),
                value: Some(value),
                state: LoadState::Pending,
            },
        );
        if let Some(key) = key {
            let channel = self.channels.entry(key).or_default();
            channel.push_back(id);
            if channel.len() == 1 {
                self.ready.push_back(id);
            }
        } else {
            self.ready.push_back(id);
        }
        Ok(id)
    }

    pub fn take_ready(&mut self) -> Option<ScheduledLoad<T, K>> {
        while let Some(id) = self.ready.pop_front() {
            let Some(entry) = self.entries.get_mut(&id) else {
                continue;
            };
            if entry.state != LoadState::Pending {
                continue;
            }
            entry.state = LoadState::Running;
            return Some(ScheduledLoad {
                id,
                key: entry.key.clone(),
                value: entry.value.take()?,
            });
        }
        None
    }

    /// Completes running work and makes the next same-key item ready.
    ///
    /// # Errors
    ///
    /// Returns an error when the load is unknown or is not running.
    pub fn complete(&mut self, id: LoadId) -> Result<(), LoadError> {
        let (key, terminal_state) = {
            let entry = self.entries.get(&id).ok_or(LoadError::UnknownLoad)?;
            if !matches!(entry.state, LoadState::Running | LoadState::Cancelled) {
                return Err(LoadError::NotRunning);
            }
            (entry.key.clone(), entry.state)
        };
        self.entries.remove(&id);
        self.advance_channel(key.as_ref(), id);
        self.record_terminal(
            id,
            if terminal_state == LoadState::Cancelled {
                LoadState::Cancelled
            } else {
                LoadState::Complete
            },
        );
        Ok(())
    }

    /// Cancels pending or running work, returning whether cancellation was accepted.
    ///
    /// # Errors
    ///
    /// Returns [`LoadError::UnknownLoad`] when the identifier is not registered.
    pub fn cancel(&mut self, id: LoadId) -> Result<bool, LoadError> {
        self.cancel_take(id).map(|cancelled| cancelled.is_some())
    }

    /// Cancels work and returns a pending value to its owner for terminal cleanup.
    ///
    /// Running work is marked cancelled but remains its channel head until [`Self::complete`]
    /// acknowledges that the worker has exited.
    ///
    /// # Errors
    ///
    /// Returns [`LoadError::UnknownLoad`] when the identifier is not registered.
    pub fn cancel_take(&mut self, id: LoadId) -> Result<Option<CancelledLoad<T>>, LoadError> {
        if self.terminal.iter().any(|(candidate, _)| *candidate == id) {
            return Ok(None);
        }
        let state = self
            .entries
            .get(&id)
            .map(|entry| entry.state)
            .ok_or(LoadError::UnknownLoad)?;
        match state {
            LoadState::Pending => {
                let entry = self.entries.remove(&id).ok_or(LoadError::UnknownLoad)?;
                self.advance_channel(entry.key.as_ref(), id);
                self.record_terminal(id, LoadState::Cancelled);
                Ok(Some(CancelledLoad {
                    previous_state: LoadState::Pending,
                    value: entry.value,
                }))
            }
            LoadState::Running => {
                self.entries
                    .get_mut(&id)
                    .ok_or(LoadError::UnknownLoad)?
                    .state = LoadState::Cancelled;
                Ok(Some(CancelledLoad {
                    previous_state: LoadState::Running,
                    value: None,
                }))
            }
            LoadState::Cancelled | LoadState::Complete => Ok(None),
        }
    }

    #[must_use]
    pub fn state(&self, id: LoadId) -> Option<LoadState> {
        self.entries.get(&id).map(|entry| entry.state).or_else(|| {
            self.terminal
                .iter()
                .rev()
                .find_map(|(candidate, state)| (*candidate == id).then_some(*state))
        })
    }

    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.entries.len()
    }

    pub fn shutdown(&mut self) {
        drop(self.shutdown_take_pending());
    }

    /// Rejects future submissions, removes pending values, and marks running work cancelled.
    #[must_use]
    pub fn shutdown_take_pending(&mut self) -> Vec<T> {
        if self.shutdown {
            return Vec::new();
        }
        self.shutdown = true;
        self.ready.clear();
        let ids = self.entries.keys().copied().collect::<Vec<_>>();
        let mut pending = Vec::new();
        for id in ids {
            let state = self.entries.get(&id).map(|entry| entry.state);
            match state {
                Some(LoadState::Pending) => {
                    if let Some(entry) = self.entries.remove(&id) {
                        if let Some(value) = entry.value {
                            pending.push(value);
                        }
                        self.record_terminal(id, LoadState::Cancelled);
                    }
                }
                Some(LoadState::Running) => {
                    if let Some(entry) = self.entries.get_mut(&id) {
                        entry.state = LoadState::Cancelled;
                    }
                }
                Some(LoadState::Cancelled | LoadState::Complete) | None => {}
            }
        }
        self.channels.clear();
        pending
    }

    fn advance_channel(&mut self, key: Option<&K>, id: LoadId) {
        let Some(key) = key else {
            return;
        };
        let mut remove = false;
        if let Some(channel) = self.channels.get_mut(key) {
            let removed_front = if channel.front() == Some(&id) {
                channel.pop_front();
                true
            } else if let Some(index) = channel.iter().position(|candidate| *candidate == id) {
                channel.remove(index);
                false
            } else {
                false
            };
            if removed_front {
                while let Some(next) = channel.front() {
                    if self
                        .entries
                        .get(next)
                        .is_some_and(|entry| entry.state == LoadState::Pending)
                    {
                        self.ready.push_back(*next);
                        break;
                    }
                    channel.pop_front();
                }
            }
            remove = channel.is_empty();
        }
        if remove {
            self.channels.remove(key);
        }
    }

    fn record_terminal(&mut self, id: LoadId, state: LoadState) {
        if self.maximum_pending == 0 {
            return;
        }
        if self.terminal.len() == self.maximum_pending {
            self.terminal.pop_front();
        }
        self.terminal.push_back((id, state));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_keys_are_serial_and_other_keys_are_independent() {
        let mut scheduler = LoadScheduler::new(4, 2, 2);
        let a = LoadKey::new("a", 8).unwrap();
        let b = LoadKey::new("b", 8).unwrap();
        let first = scheduler.submit(Some(a.clone()), 1).unwrap();
        let second = scheduler.submit(Some(a), 2).unwrap();
        let third = scheduler.submit(Some(b), 3).unwrap();

        assert_eq!(scheduler.take_ready().map(|load| load.value), Some(1));
        assert_eq!(scheduler.take_ready().map(|load| load.value), Some(3));
        assert!(scheduler.take_ready().is_none());
        scheduler.complete(first).unwrap();
        assert_eq!(scheduler.take_ready().map(|load| load.value), Some(2));
        assert_eq!(scheduler.state(second), Some(LoadState::Running));
        assert_eq!(scheduler.state(third), Some(LoadState::Running));
    }

    #[test]
    fn cancellation_promotes_the_next_equal_key_and_shutdown_rejects_work() {
        let mut scheduler = LoadScheduler::new(2, 1, 2);
        let key = LoadKey::new("shared", 8).unwrap();
        let first = scheduler.submit(Some(key.clone()), 1).unwrap();
        let second = scheduler.submit(Some(key), 2).unwrap();
        assert!(scheduler.cancel(first).unwrap());
        assert_eq!(scheduler.take_ready().map(|load| load.value), Some(2));
        scheduler.shutdown();
        assert_eq!(scheduler.state(second), Some(LoadState::Cancelled));
        assert_eq!(scheduler.submit(None, 3), Err(LoadError::Shutdown));
    }

    #[test]
    fn cancelling_queued_work_keeps_the_running_channel_head() {
        let mut scheduler = LoadScheduler::new(3, 1, 3);
        let key = LoadKey::new("shared", 8).unwrap();
        let first = scheduler.submit(Some(key.clone()), 1).unwrap();
        let second = scheduler.submit(Some(key.clone()), 2).unwrap();
        let third = scheduler.submit(Some(key), 3).unwrap();

        assert_eq!(scheduler.take_ready().map(|load| load.id), Some(first));
        assert!(scheduler.cancel(second).unwrap());
        assert!(scheduler.take_ready().is_none());
        scheduler.complete(first).unwrap();
        assert_eq!(scheduler.take_ready().map(|load| load.id), Some(third));
    }

    #[test]
    fn terminal_state_history_is_bounded() {
        let mut scheduler = LoadScheduler::new(2, 1, 2);
        let mut ids = Vec::new();
        for value in 0..10 {
            let id = scheduler.submit(None, value).unwrap();
            assert_eq!(scheduler.take_ready().map(|load| load.id), Some(id));
            scheduler.complete(id).unwrap();
            ids.push(id);
        }

        assert_eq!(scheduler.entries.len(), 0);
        assert_eq!(scheduler.terminal.len(), 2);
        assert_eq!(scheduler.state(ids[7]), None);
        assert_eq!(scheduler.state(ids[8]), Some(LoadState::Complete));
        assert_eq!(scheduler.state(ids[9]), Some(LoadState::Complete));
    }

    #[test]
    fn every_configured_bound_is_enforced() {
        let mut scheduler = LoadScheduler::new(2, 1, 1);
        let a = LoadKey::new("a", 1).unwrap();
        let b = LoadKey::new("b", 1).unwrap();
        scheduler.submit(Some(a.clone()), 1).unwrap();
        assert_eq!(scheduler.submit(Some(a), 2), Err(LoadError::ChannelFull));
        assert_eq!(
            scheduler.submit(Some(b), 2),
            Err(LoadError::TooManyChannels)
        );
        scheduler.submit(None, 2).unwrap();
        assert_eq!(scheduler.submit(None, 3), Err(LoadError::QueueFull));
        assert_eq!(LoadKey::new("", 1), Err(LoadError::InvalidKey));
    }
}
