use crate::TrackInfo;
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Debug, Default)]
pub struct SourceCancellation {
    cancelled: Arc<AtomicBool>,
}

impl SourceCancellation {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceRegistrationId(u64);

impl SourceRegistrationId {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    #[must_use]
    pub const fn from_opaque(value: u64) -> Self {
        Self(value)
    }
}

/// Explicit bounds for source registration and selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceRegistryLimits {
    pub registrations: usize,
    pub source_name_bytes: usize,
    pub reference_identifier_bytes: usize,
    pub source_detail_bytes: usize,
    pub selection_passes: usize,
}

impl Default for SourceRegistryLimits {
    fn default() -> Self {
        Self {
            registrations: 256,
            source_name_bytes: usize::from(u16::MAX),
            reference_identifier_bytes: 64 << 10,
            source_detail_bytes: 1 << 20,
            selection_passes: 5,
        }
    }
}

/// Source-neutral equivalent of Lavaplayer's `AudioReference` selection fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceReference {
    identifier: Option<String>,
    has_container_descriptor: bool,
}

impl SourceReference {
    #[must_use]
    pub fn new(identifier: Option<String>, has_container_descriptor: bool) -> Self {
        Self {
            identifier,
            has_container_descriptor,
        }
    }

    #[must_use]
    pub fn identifier(&self) -> Option<&str> {
        self.identifier.as_deref()
    }

    #[must_use]
    pub const fn has_container_descriptor(&self) -> bool {
        self.has_container_descriptor
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceLoad<T> {
    Item(T),
    Referral(SourceReference),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedSourceItem<T> {
    pub registration: SourceRegistrationId,
    pub item: T,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceDetails {
    pub source_name: String,
    pub payload: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceRegistryError {
    RegistryFull,
    InvalidSourceName,
    InvalidReference,
    SourceDetailsTooLarge,
    UnknownRegistration,
    NotEncodable,
    SourceFailure,
    Shutdown,
}

impl fmt::Display for SourceRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::RegistryFull => "source registration limit reached",
            Self::InvalidSourceName => "source name is empty or exceeds its byte limit",
            Self::InvalidReference => "source reference exceeds its identifier byte limit",
            Self::SourceDetailsTooLarge => "source-specific track details exceed their byte limit",
            Self::UnknownRegistration => "source registration is unknown",
            Self::NotEncodable => "source item is not safely encodable",
            Self::SourceFailure => "source manager failed",
            Self::Shutdown => "source registry is shut down",
        })
    }
}

impl std::error::Error for SourceRegistryError {}

/// JVM-independent source extension contract.
pub trait SourceManager<T>: Send + Sync {
    fn source_name(&self) -> &str;

    fn is_probing(&self) -> bool {
        false
    }

    /// Returns `Ok(None)` when this manager does not recognize the reference.
    ///
    /// # Errors
    ///
    /// Returns a classified source failure when recognition or loading fails.
    fn load(
        &self,
        reference: &SourceReference,
    ) -> Result<Option<SourceLoad<T>>, SourceRegistryError>;

    /// Loads with a cooperative cancellation signal.
    ///
    /// The default preserves compatibility for immediate managers. Blocking managers should
    /// override this and check the signal at every bounded I/O or decode checkpoint.
    ///
    /// # Errors
    ///
    /// Returns a classified source failure when recognition or loading fails.
    fn load_with_cancellation(
        &self,
        reference: &SourceReference,
        cancellation: &SourceCancellation,
    ) -> Result<Option<SourceLoad<T>>, SourceRegistryError> {
        if cancellation.is_cancelled() {
            return Ok(None);
        }
        self.load(reference)
    }

    fn is_encodable(&self, _item: &T) -> bool {
        true
    }

    /// Encodes source-owned track state.
    ///
    /// # Errors
    ///
    /// Returns a classified source failure when the item cannot be encoded.
    fn encode(&self, item: &T) -> Result<Vec<u8>, SourceRegistryError>;

    /// Reconstructs source-owned track state.
    ///
    /// # Errors
    ///
    /// Returns a classified source failure for invalid or unsupported payload data.
    fn decode(&self, payload: &[u8]) -> Result<T, SourceRegistryError>;

    /// Reconstructs source-owned state with the outer track metadata available.
    ///
    /// The default preserves sources whose detail payload is self-contained. Sources whose
    /// runtime item owns common track metadata should override this method.
    ///
    /// # Errors
    ///
    /// Returns a classified source failure for invalid or unsupported payload data.
    fn decode_with_info(
        &self,
        _info: &TrackInfo,
        payload: &[u8],
    ) -> Result<T, SourceRegistryError> {
        self.decode(payload)
    }

    fn shutdown(&self);
}

struct Registration<T> {
    id: SourceRegistrationId,
    name: String,
    manager: Arc<dyn SourceManager<T>>,
}

impl<T> Clone for Registration<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
            manager: Arc::clone(&self.manager),
        }
    }
}

/// Bounded append-only source registry preserving Lavaplayer selection order.
pub struct SourceRegistry<T> {
    limits: SourceRegistryLimits,
    next_id: u64,
    registrations: Vec<Registration<T>>,
    shutdown: Arc<AtomicBool>,
}

impl<T> SourceRegistry<T> {
    #[must_use]
    pub fn new(limits: SourceRegistryLimits) -> Self {
        Self {
            limits,
            next_id: 1,
            registrations: Vec::new(),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Appends a source without deduplicating its type or name.
    ///
    /// # Errors
    ///
    /// Returns a lifecycle, validation, or registration-bound error.
    pub fn register(
        &mut self,
        manager: Box<dyn SourceManager<T>>,
    ) -> Result<SourceRegistrationId, SourceRegistryError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(SourceRegistryError::Shutdown);
        }
        if self.registrations.len() >= self.limits.registrations {
            return Err(SourceRegistryError::RegistryFull);
        }
        let name = manager.source_name();
        if name.is_empty() || name.len() > self.limits.source_name_bytes {
            return Err(SourceRegistryError::InvalidSourceName);
        }

        let id = SourceRegistrationId(self.next_id);
        self.next_id = self.next_id.checked_add(1).unwrap_or(1);
        self.registrations.push(Registration {
            id,
            name: name.to_owned(),
            manager: Arc::from(manager),
        });
        Ok(id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.registrations.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.registrations.is_empty()
    }

    #[must_use]
    pub fn source_name(&self, id: SourceRegistrationId) -> Option<&str> {
        self.registration(id).map(|entry| entry.name.as_str())
    }

    /// Scans sources in insertion order and follows bounded referrals from the start.
    ///
    /// # Errors
    ///
    /// Returns immediately on source failure, invalid input, or shutdown.
    pub fn load(
        &self,
        reference: &SourceReference,
    ) -> Result<Option<LoadedSourceItem<T>>, SourceRegistryError> {
        self.load_with_cancellation(reference, &SourceCancellation::new())
    }

    /// Scans sources with a cooperative cancellation signal.
    ///
    /// # Errors
    ///
    /// Returns immediately on source failure, invalid input, or shutdown.
    pub fn load_with_cancellation(
        &self,
        reference: &SourceReference,
        cancellation: &SourceCancellation,
    ) -> Result<Option<LoadedSourceItem<T>>, SourceRegistryError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(SourceRegistryError::Shutdown);
        }
        self.validate_reference(reference)?;
        let Some(_) = reference.identifier() else {
            return Ok(None);
        };
        let mut current = reference.clone();

        'selection: for _ in 0..self.limits.selection_passes {
            if cancellation.is_cancelled() {
                return Ok(None);
            }
            for registration in &self.registrations {
                if cancellation.is_cancelled() {
                    return Ok(None);
                }
                if current.has_container_descriptor() && !registration.manager.is_probing() {
                    continue;
                }
                match registration
                    .manager
                    .load_with_cancellation(&current, cancellation)?
                {
                    None => {}
                    Some(SourceLoad::Item(item)) => {
                        return Ok(Some(LoadedSourceItem {
                            registration: registration.id,
                            item,
                        }));
                    }
                    Some(SourceLoad::Referral(reference)) => {
                        self.validate_reference(&reference)?;
                        if reference.identifier().is_none() {
                            return Ok(None);
                        }
                        current = reference;
                        continue 'selection;
                    }
                }
            }
            return Ok(None);
        }
        Ok(None)
    }

    /// Encodes with the exact registration that created the item.
    ///
    /// # Errors
    ///
    /// Returns an ownership, encodability, source, or size error.
    pub fn encode_details(
        &self,
        item: &LoadedSourceItem<T>,
    ) -> Result<SourceDetails, SourceRegistryError> {
        let registration = self
            .registration(item.registration)
            .ok_or(SourceRegistryError::UnknownRegistration)?;
        if !registration.manager.is_encodable(&item.item) {
            return Err(SourceRegistryError::NotEncodable);
        }
        let payload = registration.manager.encode(&item.item)?;
        if payload.len() > self.limits.source_detail_bytes {
            return Err(SourceRegistryError::SourceDetailsTooLarge);
        }
        Ok(SourceDetails {
            source_name: registration.name.clone(),
            payload,
        })
    }

    /// Decodes with the first registration whose name matches.
    ///
    /// # Errors
    ///
    /// Returns a source, name, or payload-size error. Unknown names return `Ok(None)`.
    pub fn decode_details(
        &self,
        details: &SourceDetails,
    ) -> Result<Option<LoadedSourceItem<T>>, SourceRegistryError> {
        self.decode_details_inner(None, details)
    }

    /// Decodes source details with the common outer track metadata.
    ///
    /// # Errors
    ///
    /// Returns a source, name, or payload-size error. Unknown names return `Ok(None)`.
    pub fn decode_details_with_info(
        &self,
        info: &TrackInfo,
        details: &SourceDetails,
    ) -> Result<Option<LoadedSourceItem<T>>, SourceRegistryError> {
        self.decode_details_inner(Some(info), details)
    }

    fn decode_details_inner(
        &self,
        info: Option<&TrackInfo>,
        details: &SourceDetails,
    ) -> Result<Option<LoadedSourceItem<T>>, SourceRegistryError> {
        if details.source_name.is_empty()
            || details.source_name.len() > self.limits.source_name_bytes
        {
            return Err(SourceRegistryError::InvalidSourceName);
        }
        if details.payload.len() > self.limits.source_detail_bytes {
            return Err(SourceRegistryError::SourceDetailsTooLarge);
        }
        let Some(registration) = self
            .registrations
            .iter()
            .find(|registration| registration.name == details.source_name)
        else {
            return Ok(None);
        };
        let item = if let Some(info) = info {
            registration
                .manager
                .decode_with_info(info, &details.payload)?
        } else {
            registration.manager.decode(&details.payload)?
        };
        Ok(Some(LoadedSourceItem {
            registration: registration.id,
            item,
        }))
    }

    /// Shuts each registration down once in insertion order.
    pub fn shutdown(&self) {
        if self.shutdown.swap(true, Ordering::AcqRel) {
            return;
        }
        self.shutdown_registrations();
    }

    pub(crate) fn load_snapshot(&self) -> Self {
        Self {
            limits: self.limits,
            next_id: self.next_id,
            registrations: self.registrations.clone(),
            shutdown: Arc::clone(&self.shutdown),
        }
    }

    pub(crate) fn begin_shutdown(&self) -> Option<Self> {
        if self.shutdown.swap(true, Ordering::AcqRel) {
            None
        } else {
            Some(self.load_snapshot())
        }
    }

    pub(crate) fn shutdown_registrations(&self) {
        for registration in &self.registrations {
            let _ = catch_unwind(AssertUnwindSafe(|| registration.manager.shutdown()));
        }
    }

    fn registration(&self, id: SourceRegistrationId) -> Option<&Registration<T>> {
        self.registrations
            .iter()
            .find(|registration| registration.id == id)
    }

    fn validate_reference(&self, reference: &SourceReference) -> Result<(), SourceRegistryError> {
        if reference
            .identifier()
            .is_some_and(|identifier| identifier.len() > self.limits.reference_identifier_bytes)
        {
            Err(SourceRegistryError::InvalidReference)
        } else {
            Ok(())
        }
    }
}
