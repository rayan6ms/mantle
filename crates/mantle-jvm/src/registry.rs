use std::fmt;

use mantle_core::{ManagerId, PlayerId, TrackId};

const INDEX_BITS: u32 = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HandleKind {
    Manager,
    Player,
    Track,
    Frame,
    Probe,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Handle(u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Entry {
    generation: u32,
    kind: HandleKind,
    core: Option<CoreObject>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CoreObject {
    Manager(ManagerId),
    Player(PlayerId),
    Track(TrackId),
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum RegistryError {
    Invalid,
    Stale,
    WrongType {
        expected: HandleKind,
        actual: HandleKind,
    },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid => formatter.write_str("invalid native handle"),
            Self::Stale => formatter.write_str("stale native handle"),
            Self::WrongType { expected, actual } => {
                write!(
                    formatter,
                    "wrong native handle type: expected {expected:?}, got {actual:?}"
                )
            }
        }
    }
}

impl std::error::Error for RegistryError {}

#[derive(Default)]
pub(crate) struct Registry {
    slots: Vec<Slot>,
    free: Vec<u32>,
    live: usize,
}

#[derive(Default)]
struct Slot {
    generation: u32,
    entry: Option<Entry>,
}

impl Registry {
    pub(crate) fn insert(&mut self, kind: HandleKind, core: Option<CoreObject>) -> Handle {
        let index = if let Some(index) = self.free.pop() {
            index
        } else {
            let index = u32::try_from(self.slots.len())
                .expect("the Gate A registry cannot exceed u32::MAX slots");
            self.slots.push(Slot::default());
            index
        };
        let slot = &mut self.slots[index as usize];
        slot.generation = slot.generation.wrapping_add(1).max(1);
        slot.entry = Some(Entry {
            generation: slot.generation,
            kind,
            core,
        });
        self.live += 1;
        Handle((u64::from(slot.generation) << INDEX_BITS) | u64::from(index))
    }

    pub(crate) fn validate(
        &self,
        handle: Handle,
        expected: HandleKind,
    ) -> Result<(), RegistryError> {
        let (index, generation) = handle.parts();
        let slot = self.slots.get(index).ok_or(RegistryError::Invalid)?;
        let entry = slot.entry.ok_or(RegistryError::Stale)?;
        if entry.generation != generation {
            return Err(RegistryError::Stale);
        }
        if entry.kind != expected {
            return Err(RegistryError::WrongType {
                expected,
                actual: entry.kind,
            });
        }
        Ok(())
    }

    pub(crate) fn core(
        &self,
        handle: Handle,
        expected: HandleKind,
    ) -> Result<CoreObject, RegistryError> {
        self.validate(handle, expected)?;
        let (index, _) = handle.parts();
        self.slots[index]
            .entry
            .and_then(|entry| entry.core)
            .ok_or(RegistryError::Invalid)
    }

    pub(crate) fn release(&mut self, handle: Handle) -> Option<CoreObject> {
        let (index, generation) = handle.parts();
        let slot = self.slots.get_mut(index)?;
        if slot
            .entry
            .is_none_or(|entry| entry.generation != generation)
        {
            return None;
        }
        let core = slot.entry.take().and_then(|entry| entry.core);
        self.free
            .push(u32::try_from(index).expect("handle index originated as u32"));
        self.live -= 1;
        core
    }

    pub(crate) fn live(&self) -> usize {
        self.live
    }
}

impl Handle {
    pub(crate) fn from_jlong(value: i64) -> Result<Self, RegistryError> {
        let raw = u64::from_ne_bytes(value.to_ne_bytes());
        (raw != 0)
            .then_some(Self(raw))
            .ok_or(RegistryError::Invalid)
    }

    pub(crate) fn as_jlong(self) -> i64 {
        i64::from_ne_bytes(self.0.to_ne_bytes())
    }

    fn parts(self) -> (usize, u32) {
        let index =
            u32::try_from(self.0 & u64::from(u32::MAX)).expect("handle index is masked to u32");
        let generation =
            u32::try_from(self.0 >> INDEX_BITS).expect("handle generation occupies the upper u32");
        (index as usize, generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_rejects_stale_handle_after_slot_reuse() {
        let mut registry = Registry::default();
        let stale = registry.insert(HandleKind::Player, None);
        assert_eq!(registry.release(stale), None);
        let current = registry.insert(HandleKind::Player, None);
        assert_ne!(stale, current);
        assert_eq!(
            registry.validate(stale, HandleKind::Player),
            Err(RegistryError::Stale)
        );
        assert_eq!(registry.validate(current, HandleKind::Player), Ok(()));
    }

    #[test]
    fn wrong_type_and_double_release_are_safe() {
        let mut registry = Registry::default();
        let handle = registry.insert(HandleKind::Track, None);
        assert!(matches!(
            registry.validate(handle, HandleKind::Player),
            Err(RegistryError::WrongType { .. })
        ));
        assert_eq!(registry.release(handle), None);
        assert_eq!(registry.release(handle), None);
        assert_eq!(registry.live(), 0);
    }
}
