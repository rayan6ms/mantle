use mantle_core::{
    LoadedSourceItem, SerializationLimits, SourceDetails, SourceLoad, SourceManager,
    SourceReference, SourceRegistrationId, SourceRegistry, SourceRegistryError,
    SourceRegistryLimits, TrackInfo, decode_source_track, encode_source_track,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone)]
enum Behavior {
    Ignore,
    Match(&'static str),
    Refer(&'static str),
    Fail,
}

struct MockSource {
    name: &'static str,
    probing: bool,
    behavior: Behavior,
    events: Arc<Mutex<Vec<String>>>,
}

impl SourceManager<String> for MockSource {
    fn source_name(&self) -> &str {
        self.name
    }

    fn is_probing(&self) -> bool {
        self.probing
    }

    fn load(
        &self,
        reference: &SourceReference,
    ) -> Result<Option<SourceLoad<String>>, SourceRegistryError> {
        self.events.lock().unwrap().push(format!(
            "load:{}:{:?}",
            self.name,
            reference.identifier()
        ));
        match self.behavior {
            Behavior::Ignore => Ok(None),
            Behavior::Match(value) => Ok(Some(SourceLoad::Item(value.to_owned()))),
            Behavior::Refer(identifier) => Ok(Some(SourceLoad::Referral(SourceReference::new(
                Some(identifier.to_owned()),
                false,
            )))),
            Behavior::Fail => Err(SourceRegistryError::SourceFailure),
        }
    }

    fn encode(&self, item: &String) -> Result<Vec<u8>, SourceRegistryError> {
        Ok(format!("{}:{item}", self.name).into_bytes())
    }

    fn decode(&self, payload: &[u8]) -> Result<String, SourceRegistryError> {
        String::from_utf8(payload.to_vec()).map_err(|_| SourceRegistryError::SourceFailure)
    }

    fn shutdown(&self) {
        self.events
            .lock()
            .unwrap()
            .push(format!("shutdown:{}", self.name));
    }
}

fn source(
    name: &'static str,
    probing: bool,
    behavior: Behavior,
    events: &Arc<Mutex<Vec<String>>>,
) -> Box<dyn SourceManager<String>> {
    Box::new(MockSource {
        name,
        probing,
        behavior,
        events: Arc::clone(events),
    })
}

#[test]
fn registration_is_append_only_and_first_non_null_result_wins() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut registry = SourceRegistry::new(SourceRegistryLimits::default());
    let first = registry
        .register(source("duplicate", false, Behavior::Ignore, &events))
        .unwrap();
    let second = registry
        .register(source(
            "duplicate",
            false,
            Behavior::Match("winner"),
            &events,
        ))
        .unwrap();
    registry
        .register(source("later", false, Behavior::Match("wrong"), &events))
        .unwrap();

    assert_ne!(first, second);
    assert_eq!(registry.len(), 3);
    let loaded = registry
        .load(&SourceReference::new(Some("id".to_owned()), false))
        .unwrap()
        .unwrap();
    assert_eq!(loaded.registration, second);
    assert_eq!(loaded.item, "winner");
    assert_eq!(
        *events.lock().unwrap(),
        ["load:duplicate:Some(\"id\")", "load:duplicate:Some(\"id\")"]
    );
}

#[test]
fn referrals_restart_at_the_first_source_and_are_bounded_to_five_passes() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut registry = SourceRegistry::new(SourceRegistryLimits::default());
    registry
        .register(source("referrer", false, Behavior::Refer("next"), &events))
        .unwrap();
    registry
        .register(source(
            "unreachable",
            false,
            Behavior::Match("wrong"),
            &events,
        ))
        .unwrap();

    assert_eq!(
        registry
            .load(&SourceReference::new(Some("start".to_owned()), false))
            .unwrap(),
        None
    );
    let observed = events.lock().unwrap();
    assert_eq!(observed.len(), 5);
    assert!(
        observed
            .iter()
            .all(|event| event.starts_with("load:referrer:"))
    );
}

#[test]
fn descriptor_references_only_call_probing_sources_and_failures_do_not_fall_through() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut registry = SourceRegistry::new(SourceRegistryLimits::default());
    registry
        .register(source("direct", false, Behavior::Match("wrong"), &events))
        .unwrap();
    registry
        .register(source("probe", true, Behavior::Fail, &events))
        .unwrap();
    registry
        .register(source("later", true, Behavior::Match("wrong"), &events))
        .unwrap();

    assert_eq!(
        registry.load(&SourceReference::new(Some("id".to_owned()), true)),
        Err(SourceRegistryError::SourceFailure)
    );
    assert_eq!(*events.lock().unwrap(), ["load:probe:Some(\"id\")"]);
}

#[test]
fn encoding_uses_the_owner_but_duplicate_name_decoding_uses_the_first_registration() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut registry = SourceRegistry::new(SourceRegistryLimits::default());
    let first = registry
        .register(source("same", false, Behavior::Ignore, &events))
        .unwrap();
    let second = registry
        .register(source("same", false, Behavior::Ignore, &events))
        .unwrap();
    let details = registry
        .encode_details(&LoadedSourceItem {
            registration: second,
            item: "payload".to_owned(),
        })
        .unwrap();
    assert_eq!(details.source_name, "same");
    assert_eq!(details.payload, b"same:payload");

    let decoded = registry.decode_details(&SourceDetails {
        source_name: "same".to_owned(),
        payload: b"decoded".to_vec(),
    });
    assert_eq!(
        decoded.unwrap(),
        Some(LoadedSourceItem {
            registration: first,
            item: "decoded".to_owned(),
        })
    );
    assert_eq!(
        registry
            .decode_details(&SourceDetails {
                source_name: "unknown".to_owned(),
                payload: Vec::new(),
            })
            .unwrap(),
        None
    );
}

#[test]
fn every_bound_and_ordered_idempotent_shutdown_are_enforced() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let limits = SourceRegistryLimits {
        registrations: 2,
        source_name_bytes: 4,
        reference_identifier_bytes: 3,
        source_detail_bytes: 3,
        selection_passes: 5,
    };
    let mut registry = SourceRegistry::new(limits);
    assert_eq!(
        registry.register(source("oversized", false, Behavior::Ignore, &events)),
        Err(SourceRegistryError::InvalidSourceName)
    );
    let one = registry
        .register(source("one", false, Behavior::Ignore, &events))
        .unwrap();
    registry
        .register(source("two", false, Behavior::Ignore, &events))
        .unwrap();
    assert_eq!(
        registry.register(source("tri", false, Behavior::Ignore, &events)),
        Err(SourceRegistryError::RegistryFull)
    );
    assert_eq!(
        registry.load(&SourceReference::new(Some("long".to_owned()), false)),
        Err(SourceRegistryError::InvalidReference)
    );
    assert_eq!(
        registry.encode_details(&LoadedSourceItem {
            registration: one,
            item: "long".to_owned(),
        }),
        Err(SourceRegistryError::SourceDetailsTooLarge)
    );

    registry.shutdown();
    registry.shutdown();
    assert_eq!(*events.lock().unwrap(), ["shutdown:one", "shutdown:two"]);
    assert_eq!(
        registry.load(&SourceReference::new(Some("id".to_owned()), false)),
        Err(SourceRegistryError::Shutdown)
    );
    assert_eq!(
        registry.register(source("new", false, Behavior::Ignore, &events)),
        Err(SourceRegistryError::Shutdown)
    );
    assert_eq!(
        registry.source_name(SourceRegistrationId::from_opaque(999)),
        None
    );
}

#[test]
fn full_wire_records_use_the_owner_for_encoding_and_first_name_match_for_decoding() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut registry = SourceRegistry::new(SourceRegistryLimits::default());
    registry
        .register(source("same", false, Behavior::Ignore, &events))
        .unwrap();
    let owner = registry
        .register(source("same", false, Behavior::Ignore, &events))
        .unwrap();
    let info = TrackInfo {
        title: "title".to_owned(),
        author: "author".to_owned(),
        duration: Duration::from_secs(12),
        identifier: "local:/track.ogg".to_owned(),
        is_stream: false,
        uri: Some("file:///track.ogg".to_owned()),
        artwork_url: None,
        isrc: Some("TEST00000001".to_owned()),
    };
    let encoded = encode_source_track(
        &info,
        Duration::from_millis(345),
        &LoadedSourceItem {
            registration: owner,
            item: "payload".to_owned(),
        },
        &registry,
        SerializationLimits::default(),
    )
    .unwrap();
    let decoded = decode_source_track(&encoded, &registry, SerializationLimits::default())
        .unwrap()
        .unwrap();

    assert_eq!(decoded.info, info);
    assert_eq!(decoded.position, Duration::from_millis(345));
    assert_eq!(decoded.item.item, "same:payload");

    let empty = SourceRegistry::<String>::new(SourceRegistryLimits::default());
    assert_eq!(
        decode_source_track(&encoded, &empty, SerializationLimits::default()).unwrap(),
        None
    );
}
