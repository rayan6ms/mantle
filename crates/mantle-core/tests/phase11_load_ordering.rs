use mantle_core::{LoadError, LoadScheduler, LoadState, OpaqueLoadKey};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct NativeKey {
    tenant: u16,
    logical_channel: u16,
}

#[test]
fn native_equality_keys_serialize_equal_values_and_keep_other_values_independent() {
    let mut scheduler = LoadScheduler::<_, NativeKey>::new_for_keys(4, 2, 2);
    let first = scheduler
        .submit(
            Some(NativeKey {
                tenant: 7,
                logical_channel: 9,
            }),
            "first",
        )
        .unwrap();
    let second = scheduler
        .submit(
            Some(NativeKey {
                tenant: 7,
                logical_channel: 9,
            }),
            "second",
        )
        .unwrap();
    let independent = scheduler
        .submit(
            Some(NativeKey {
                tenant: 7,
                logical_channel: 10,
            }),
            "independent",
        )
        .unwrap();

    assert_eq!(scheduler.take_ready().unwrap().id, first);
    assert_eq!(scheduler.take_ready().unwrap().id, independent);
    assert!(scheduler.take_ready().is_none());
    scheduler.complete(first).unwrap();
    assert_eq!(scheduler.take_ready().unwrap().id, second);
}

#[test]
fn opaque_boundary_tokens_preserve_fifo_cancellation_and_capacity() {
    let mut scheduler = LoadScheduler::<_, OpaqueLoadKey>::new_for_keys(3, 1, 3);
    let key = OpaqueLoadKey::from_opaque(41).unwrap();
    assert_eq!(OpaqueLoadKey::from_opaque(0), Err(LoadError::InvalidKey));
    let first = scheduler.submit(Some(key), 1).unwrap();
    let cancelled = scheduler.submit(Some(key), 2).unwrap();
    let last = scheduler.submit(Some(key), 3).unwrap();
    assert_eq!(scheduler.submit(Some(key), 4), Err(LoadError::QueueFull));

    assert_eq!(scheduler.take_ready().unwrap().id, first);
    assert!(scheduler.cancel(cancelled).unwrap());
    assert!(scheduler.take_ready().is_none());
    scheduler.complete(first).unwrap();
    assert_eq!(scheduler.take_ready().unwrap().id, last);
    assert_eq!(scheduler.state(cancelled), Some(LoadState::Cancelled));
}
