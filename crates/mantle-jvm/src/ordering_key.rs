use jni::objects::{JObject, JValue};
use jni::refs::Global;
use jni::{Env, jni_sig, jni_str};
use mantle_core::{LoadTerminalHook, OpaqueLoadKey};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

struct ReleaseState {
    outstanding: usize,
    queued: VecDeque<OpaqueLoadKey>,
    shutdown: bool,
}

struct ReleaseQueueInner {
    maximum_outstanding: usize,
    state: Mutex<ReleaseState>,
}

/// Bounded handoff from native loader workers to an attached JVM callback thread.
#[derive(Clone)]
pub struct JvmOrderingKeyReleaseQueue {
    inner: Arc<ReleaseQueueInner>,
}

impl JvmOrderingKeyReleaseQueue {
    #[must_use]
    pub fn new(maximum_outstanding: usize) -> Self {
        Self {
            inner: Arc::new(ReleaseQueueInner {
                maximum_outstanding,
                state: Mutex::new(ReleaseState {
                    outstanding: 0,
                    queued: VecDeque::with_capacity(maximum_outstanding),
                    shutdown: false,
                }),
            }),
        }
    }

    fn reserve(&self) -> bool {
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.shutdown || state.outstanding >= self.inner.maximum_outstanding {
            return false;
        }
        state.outstanding += 1;
        true
    }

    fn abandon(&self) {
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.outstanding = state.outstanding.saturating_sub(1);
    }

    fn enqueue(&self, key: OpaqueLoadKey) {
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.shutdown {
            return;
        }
        debug_assert!(state.queued.len() < self.inner.maximum_outstanding);
        state.queued.push_back(key);
    }

    fn take_all(&self) -> Vec<OpaqueLoadKey> {
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let releases = state.queued.drain(..).collect::<Vec<_>>();
        state.outstanding = state.outstanding.saturating_sub(releases.len());
        releases
    }

    #[must_use]
    pub fn outstanding(&self) -> usize {
        self.inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .outstanding
    }

    #[must_use]
    pub fn pending_releases(&self) -> usize {
        self.inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .queued
            .len()
    }

    /// Discards queued opaque tokens after their owning key table has been shut down.
    pub fn shutdown(&self) {
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.shutdown = true;
        state.outstanding = 0;
        state.queued.clear();
    }
}

struct JvmOrderingKeyLease {
    key: Option<OpaqueLoadKey>,
    releases: JvmOrderingKeyReleaseQueue,
}

impl JvmOrderingKeyLease {
    fn release(&mut self) {
        if let Some(key) = self.key.take() {
            self.releases.enqueue(key);
        }
    }
}

impl LoadTerminalHook for JvmOrderingKeyLease {
    fn on_terminal(mut self: Box<Self>) {
        self.release();
    }
}

impl Drop for JvmOrderingKeyLease {
    fn drop(&mut self) {
        self.release();
    }
}

struct KeyEntry<R> {
    key: OpaqueLoadKey,
    hash_code: i32,
    reference: R,
    leases: usize,
}

#[derive(Debug)]
enum KeyTableError<E> {
    Full,
    Shutdown,
    Comparison(E),
}

struct EqualityKeyTable<R> {
    maximum_keys: usize,
    next_key: u64,
    entries: Vec<KeyEntry<R>>,
    shutdown: bool,
}

impl<R> EqualityKeyTable<R> {
    const fn new(maximum_keys: usize) -> Self {
        Self {
            maximum_keys,
            next_key: 1,
            entries: Vec::new(),
            shutdown: false,
        }
    }

    fn acquire<E>(
        &mut self,
        hash_code: i32,
        reference: R,
        mut equals: impl FnMut(&R, &R) -> Result<bool, E>,
    ) -> Result<OpaqueLoadKey, KeyTableError<E>> {
        if self.shutdown {
            return Err(KeyTableError::Shutdown);
        }
        for index in 0..self.entries.len() {
            if self.entries[index].hash_code != hash_code {
                continue;
            }
            let equal = equals(&reference, &self.entries[index].reference)
                .map_err(KeyTableError::Comparison)?;
            if equal {
                self.entries[index].leases = self.entries[index]
                    .leases
                    .checked_add(1)
                    .ok_or(KeyTableError::Full)?;
                return Ok(self.entries[index].key);
            }
        }
        if self.entries.len() >= self.maximum_keys {
            return Err(KeyTableError::Full);
        }

        let key = self.allocate_key();
        self.entries.push(KeyEntry {
            key,
            hash_code,
            reference,
            leases: 1,
        });
        Ok(key)
    }

    fn release(&mut self, key: OpaqueLoadKey) -> bool {
        let Some(index) = self.entries.iter().position(|entry| entry.key == key) else {
            return false;
        };
        if self.entries[index].leases > 1 {
            self.entries[index].leases -= 1;
        } else {
            self.entries.remove(index);
        }
        true
    }

    fn shutdown(&mut self) {
        self.shutdown = true;
        self.entries.clear();
    }

    fn allocate_key(&mut self) -> OpaqueLoadKey {
        loop {
            let raw = self.next_key.max(1);
            self.next_key = raw.checked_add(1).unwrap_or(1);
            let key = OpaqueLoadKey::from_opaque(raw).expect("non-zero key is constructed");
            if self.entries.iter().all(|entry| entry.key != key) {
                return key;
            }
        }
    }
}

/// Bounded table adapting Java `hashCode`/`equals` keys to stable native load-channel tokens.
///
/// One global reference is retained per distinct live equality key, not per queued load. Each
/// successful acquisition must be paired with [`Self::release`] when that load completes or is
/// cancelled. Dropping/releasing references should occur on an attached JVM thread.
pub struct JvmOrderingKeyTable {
    keys: EqualityKeyTable<Global<JObject<'static>>>,
}

impl JvmOrderingKeyTable {
    #[must_use]
    pub const fn new(maximum_keys: usize) -> Self {
        Self {
            keys: EqualityKeyTable::new(maximum_keys),
        }
    }

    /// Acquires the stable token for a non-null Java ordering key.
    ///
    /// Java identity is checked before `equals`, and `hashCode` narrows comparisons, matching the
    /// effective key behavior of Lavaplayer's `ConcurrentHashMap<Object, ...>`.
    ///
    /// # Errors
    ///
    /// Returns a JNI error for null keys, Java exceptions, exhausted capacity, or shutdown.
    pub fn acquire<'local>(
        &mut self,
        env: &mut Env<'local>,
        key: &JObject<'local>,
    ) -> jni::errors::Result<OpaqueLoadKey> {
        if key.is_null() {
            return Err(jni::errors::Error::NullPtr(
                "ordered load key must not be null",
            ));
        }
        let hash_code = env
            .call_method(key, jni_str!("hashCode"), jni_sig!("()I"), &[])?
            .i()?;
        let reference = env.new_global_ref(key)?;
        self.keys
            .acquire(hash_code, reference, |candidate, existing| {
                if env.is_same_object(candidate, existing)? {
                    return Ok(true);
                }
                env.call_method(
                    candidate,
                    jni_str!("equals"),
                    jni_sig!("(Ljava/lang/Object;)Z"),
                    &[JValue::Object(existing.as_ref())],
                )?
                .z()
            })
            .map_err(|error| match error {
                KeyTableError::Full => {
                    jni::errors::Error::NullPtr("ordered load key capacity reached")
                }
                KeyTableError::Shutdown => {
                    jni::errors::Error::NullPtr("ordered load key table is shut down")
                }
                KeyTableError::Comparison(error) => error,
            })
    }

    /// Acquires a token and a one-shot terminal hook for one ordered load.
    ///
    /// The hook may run on a native loader worker; it only queues the opaque token. Call
    /// [`Self::drain_releases`] from an attached JVM thread to release global references.
    ///
    /// # Errors
    ///
    /// Returns a JNI error for key comparison/acquisition failures or a full release handoff.
    pub fn acquire_for_load<'local>(
        &mut self,
        env: &mut Env<'local>,
        key: &JObject<'local>,
        releases: &JvmOrderingKeyReleaseQueue,
    ) -> jni::errors::Result<(OpaqueLoadKey, Box<dyn LoadTerminalHook>)> {
        if !releases.reserve() {
            return Err(jni::errors::Error::NullPtr(
                "ordered load release capacity reached",
            ));
        }
        match self.acquire(env, key) {
            Ok(key) => Ok((
                key,
                Box::new(JvmOrderingKeyLease {
                    key: Some(key),
                    releases: releases.clone(),
                }),
            )),
            Err(error) => {
                releases.abandon();
                Err(error)
            }
        }
    }

    /// Releases all worker-completed leases on the caller's attached JVM thread.
    pub fn drain_releases(
        &mut self,
        _env: &mut Env<'_>,
        releases: &JvmOrderingKeyReleaseQueue,
    ) -> usize {
        releases
            .take_all()
            .into_iter()
            .filter(|key| self.release(*key))
            .count()
    }

    /// Releases one load's lease and its last global reference when the channel becomes idle.
    #[must_use]
    pub fn release(&mut self, key: OpaqueLoadKey) -> bool {
        self.keys.release(key)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.keys.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keys.entries.is_empty()
    }

    /// Rejects future acquisitions and drops all retained global references.
    pub fn shutdown(&mut self) {
        self.keys.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockReference {
        equality_group: u8,
        drops: Arc<AtomicUsize>,
    }

    impl Drop for MockReference {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn reference(equality_group: u8, drops: &Arc<AtomicUsize>) -> MockReference {
        MockReference {
            equality_group,
            drops: Arc::clone(drops),
        }
    }

    #[test]
    fn equal_objects_share_one_token_and_reference_until_the_last_lease() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut table = EqualityKeyTable::new(2);
        let first = table
            .acquire(7, reference(1, &drops), |candidate, existing| {
                Ok::<_, ()>(candidate.equality_group == existing.equality_group)
            })
            .unwrap();
        let equal = table
            .acquire(7, reference(1, &drops), |candidate, existing| {
                Ok::<_, ()>(candidate.equality_group == existing.equality_group)
            })
            .unwrap();

        assert_eq!(first, equal);
        assert_eq!(table.entries.len(), 1);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
        assert!(table.release(first));
        assert_eq!(table.entries.len(), 1);
        assert!(table.release(equal));
        assert_eq!(table.entries.len(), 0);
        assert_eq!(drops.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn hash_collisions_stay_distinct_and_capacity_is_bounded() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut table = EqualityKeyTable::new(2);
        let first = table
            .acquire(9, reference(1, &drops), |candidate, existing| {
                Ok::<_, ()>(candidate.equality_group == existing.equality_group)
            })
            .unwrap();
        let collision = table
            .acquire(9, reference(2, &drops), |candidate, existing| {
                Ok::<_, ()>(candidate.equality_group == existing.equality_group)
            })
            .unwrap();
        let full = table.acquire(10, reference(3, &drops), |_, _| Ok::<_, ()>(false));

        assert_ne!(first, collision);
        assert!(matches!(full, Err(KeyTableError::Full)));
        assert_eq!(table.entries.len(), 2);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn comparison_failure_is_transactional_and_shutdown_releases_and_rejects() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut table = EqualityKeyTable::new(2);
        table
            .acquire(3, reference(1, &drops), |_, _| Ok::<_, &'static str>(false))
            .unwrap();
        let failed = table.acquire(3, reference(2, &drops), |_, _| Err("equals failed"));
        assert!(matches!(
            failed,
            Err(KeyTableError::Comparison("equals failed"))
        ));
        assert_eq!(table.entries.len(), 1);

        table.shutdown();
        assert_eq!(table.entries.len(), 0);
        assert_eq!(drops.load(Ordering::Relaxed), 2);
        let rejected = table.acquire(1, reference(3, &drops), |_, _| Ok::<_, ()>(false));
        assert!(matches!(rejected, Err(KeyTableError::Shutdown)));
        assert_eq!(drops.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn terminal_hooks_use_a_bounded_release_handoff_and_fire_once() {
        let releases = JvmOrderingKeyReleaseQueue::new(1);
        assert!(releases.reserve());
        assert!(!releases.reserve());
        let key = OpaqueLoadKey::from_opaque(7).unwrap();
        let hook: Box<dyn LoadTerminalHook> = Box::new(JvmOrderingKeyLease {
            key: Some(key),
            releases: releases.clone(),
        });

        hook.on_terminal();
        assert_eq!(releases.outstanding(), 1);
        assert_eq!(releases.pending_releases(), 1);
        assert_eq!(releases.take_all(), [key]);
        assert_eq!(releases.outstanding(), 0);
        assert_eq!(releases.pending_releases(), 0);
    }
}
