use mantle_core::{
    LoadExecutorLimits, LoadHandleState, LoadKey, LoadTerminalHook, LoadedSourceItem, SourceLoad,
    SourceLoadExecutor, SourceLoadFailure, SourceLoadResult, SourceLoadResultHandler,
    SourceManager, SourceReference, SourceRegistry, SourceRegistryError, SourceRegistryLimits,
    dispatch_source_load,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

struct Callback<F>(Option<F>);

impl<T, F> SourceLoadResultHandler<T> for Callback<F>
where
    F: FnOnce(SourceLoadResult<T>) + Send + 'static,
{
    fn finished(mut self: Box<Self>, result: SourceLoadResult<T>) {
        self.0.take().unwrap()(result);
    }
}

fn callback<T>(
    callback: impl FnOnce(SourceLoadResult<T>) + Send + 'static,
) -> Box<dyn SourceLoadResultHandler<T>> {
    Box::new(Callback(Some(callback)))
}

struct Hook(Arc<AtomicUsize>);

impl LoadTerminalHook for Hook {
    fn on_terminal(self: Box<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

struct ImmediateSource;

impl SourceManager<String> for ImmediateSource {
    fn source_name(&self) -> &'static str {
        "immediate"
    }

    fn load(
        &self,
        reference: &SourceReference,
    ) -> Result<Option<SourceLoad<String>>, SourceRegistryError> {
        match reference.identifier() {
            Some("hit") => Ok(Some(SourceLoad::Item("loaded".to_owned()))),
            Some("fail") => Err(SourceRegistryError::SourceFailure),
            _ => Ok(None),
        }
    }

    fn encode(&self, item: &String) -> Result<Vec<u8>, SourceRegistryError> {
        Ok(item.as_bytes().to_vec())
    }

    fn decode(&self, payload: &[u8]) -> Result<String, SourceRegistryError> {
        String::from_utf8(payload.to_vec()).map_err(|_| SourceRegistryError::SourceFailure)
    }

    fn shutdown(&self) {}
}

fn immediate_registry() -> SourceRegistry<String> {
    let mut registry = SourceRegistry::new(SourceRegistryLimits::default());
    registry.register(Box::new(ImmediateSource)).unwrap();
    registry
}

#[test]
fn executor_accepts_source_registration_after_loading_has_started() {
    let registry = SourceRegistry::new(SourceRegistryLimits::default());
    let mut executor = SourceLoadExecutor::<_, LoadKey>::new(
        registry,
        LoadExecutorLimits {
            workers: 1,
            maximum_pending: 2,
            maximum_channels: 1,
            maximum_per_channel: 1,
        },
    )
    .unwrap();
    let (missing_tx, missing_rx) = mpsc::channel();
    let missing = executor.submit(
        SourceReference::new(Some("hit".to_owned()), false),
        callback(move |result| missing_tx.send(result).unwrap()),
    );

    assert_eq!(missing.wait(), LoadHandleState::Complete);
    assert_eq!(
        missing_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
        SourceLoadResult::NoMatches
    );

    executor.register_source(Box::new(ImmediateSource)).unwrap();
    let (loaded_tx, loaded_rx) = mpsc::channel();
    let loaded = executor.submit(
        SourceReference::new(Some("hit".to_owned()), false),
        callback(move |result| loaded_tx.send(result).unwrap()),
    );

    assert_eq!(loaded.wait(), LoadHandleState::Complete);
    assert!(matches!(
        loaded_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
        SourceLoadResult::Item(LoadedSourceItem { item, .. }) if item == "loaded"
    ));
    executor.shutdown();
}

#[test]
fn synchronous_dispatch_reports_each_terminal_shape_exactly_once() {
    let registry = immediate_registry();
    let observed = Arc::new(Mutex::new(Vec::new()));

    for identifier in ["hit", "miss", "fail"] {
        let observed = Arc::clone(&observed);
        dispatch_source_load(
            &registry,
            &SourceReference::new(Some(identifier.to_owned()), false),
            callback(move |result| observed.lock().unwrap().push(result)),
        );
    }

    let observed = observed.lock().unwrap();
    assert_eq!(observed.len(), 3);
    assert!(matches!(
        &observed[0],
        SourceLoadResult::Item(LoadedSourceItem { item, .. }) if item == "loaded"
    ));
    assert_eq!(observed[1], SourceLoadResult::NoMatches);
    assert_eq!(
        observed[2],
        SourceLoadResult::Failed(SourceLoadFailure::Source(
            SourceRegistryError::SourceFailure
        ))
    );
}

struct ControlledSource {
    starts: mpsc::Sender<String>,
    releases: Mutex<mpsc::Receiver<()>>,
    calls: Arc<AtomicUsize>,
}

impl SourceManager<String> for ControlledSource {
    fn source_name(&self) -> &'static str {
        "controlled"
    }

    fn load(
        &self,
        _reference: &SourceReference,
    ) -> Result<Option<SourceLoad<String>>, SourceRegistryError> {
        unreachable!("the cancellation-aware path is required")
    }

    fn load_with_cancellation(
        &self,
        reference: &SourceReference,
        cancellation: &mantle_core::SourceCancellation,
    ) -> Result<Option<SourceLoad<String>>, SourceRegistryError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let identifier = reference.identifier().unwrap().to_owned();
        self.starts.send(identifier.clone()).unwrap();
        while !cancellation.is_cancelled() {
            match self
                .releases
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_millis(5))
            {
                Ok(()) => return Ok(Some(SourceLoad::Item(identifier))),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(None),
            }
        }
        Ok(None)
    }

    fn encode(&self, item: &String) -> Result<Vec<u8>, SourceRegistryError> {
        Ok(item.as_bytes().to_vec())
    }

    fn decode(&self, payload: &[u8]) -> Result<String, SourceRegistryError> {
        String::from_utf8(payload.to_vec()).map_err(|_| SourceRegistryError::SourceFailure)
    }

    fn shutdown(&self) {}
}

fn controlled_registry(
    starts: mpsc::Sender<String>,
    releases: mpsc::Receiver<()>,
    calls: Arc<AtomicUsize>,
) -> SourceRegistry<String> {
    let mut registry = SourceRegistry::new(SourceRegistryLimits::default());
    registry
        .register(Box::new(ControlledSource {
            starts,
            releases: Mutex::new(releases),
            calls,
        }))
        .unwrap();
    registry
}

#[test]
fn registration_does_not_wait_for_a_running_source_callback() {
    let (starts_tx, starts_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let registry = controlled_registry(starts_tx, release_rx, Arc::new(AtomicUsize::new(0)));
    let executor = Arc::new(
        SourceLoadExecutor::<_, LoadKey>::new(
            registry,
            LoadExecutorLimits {
                workers: 1,
                maximum_pending: 2,
                maximum_channels: 1,
                maximum_per_channel: 1,
            },
        )
        .unwrap(),
    );
    let running = executor.submit(
        SourceReference::new(Some("running".to_owned()), false),
        callback(|_| {}),
    );
    assert_eq!(
        starts_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
        "running"
    );

    let (registered_tx, registered_rx) = mpsc::channel();
    let registration_executor = Arc::clone(&executor);
    let registration = std::thread::spawn(move || {
        let result = registration_executor.register_source(Box::new(ImmediateSource));
        registered_tx.send(result).unwrap();
    });
    let result = registered_rx.recv_timeout(Duration::from_millis(250));
    release_tx.send(()).unwrap();
    registration.join().unwrap();
    assert!(
        result
            .expect("registration blocked on running source")
            .is_ok()
    );
    assert_eq!(running.wait(), LoadHandleState::Complete);

    let mut executor = Arc::try_unwrap(executor)
        .ok()
        .expect("executor still shared");
    executor.shutdown();
}

#[test]
fn async_equal_keys_are_fifo_while_unequal_keys_can_run_concurrently() {
    let (starts_tx, starts_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let calls = Arc::new(AtomicUsize::new(0));
    let registry = controlled_registry(starts_tx, release_rx, Arc::clone(&calls));
    let mut executor = SourceLoadExecutor::<_, LoadKey>::new(
        registry,
        LoadExecutorLimits {
            workers: 2,
            maximum_pending: 3,
            maximum_channels: 2,
            maximum_per_channel: 2,
        },
    )
    .unwrap();
    let results = Arc::new(Mutex::new(Vec::new()));
    let same = LoadKey::new("same", 8).unwrap();
    let other = LoadKey::new("other", 8).unwrap();

    let first = executor.submit_ordered(
        same.clone(),
        SourceReference::new(Some("first".to_owned()), false),
        result_collector(&results),
    );
    let second = executor.submit_ordered(
        same,
        SourceReference::new(Some("second".to_owned()), false),
        result_collector(&results),
    );
    let independent = executor.submit_ordered(
        other,
        SourceReference::new(Some("independent".to_owned()), false),
        result_collector(&results),
    );

    let mut initial = [
        starts_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
        starts_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
    ];
    initial.sort();
    assert_eq!(initial, ["first", "independent"]);
    assert!(starts_rx.recv_timeout(Duration::from_millis(50)).is_err());

    release_tx.send(()).unwrap();
    release_tx.send(()).unwrap();
    assert_eq!(
        starts_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
        "second"
    );
    release_tx.send(()).unwrap();

    assert_eq!(first.wait(), LoadHandleState::Complete);
    assert_eq!(second.wait(), LoadHandleState::Complete);
    assert_eq!(independent.wait(), LoadHandleState::Complete);
    assert_eq!(calls.load(Ordering::Relaxed), 3);
    assert_eq!(results.lock().unwrap().len(), 3);
    executor.shutdown();
}

fn result_collector(
    results: &Arc<Mutex<Vec<SourceLoadResult<String>>>>,
) -> Box<dyn SourceLoadResultHandler<String>> {
    let results = Arc::clone(results);
    callback(move |result| results.lock().unwrap().push(result))
}

#[test]
fn pending_and_running_cancellation_suppress_selection_or_callbacks_and_release_leases() {
    let (starts_tx, starts_rx) = mpsc::channel();
    let (_release_tx, release_rx) = mpsc::channel();
    let calls = Arc::new(AtomicUsize::new(0));
    let registry = controlled_registry(starts_tx, release_rx, Arc::clone(&calls));
    let mut executor = SourceLoadExecutor::<_, LoadKey>::new(
        registry,
        LoadExecutorLimits {
            workers: 1,
            maximum_pending: 2,
            maximum_channels: 1,
            maximum_per_channel: 2,
        },
    )
    .unwrap();
    let callbacks = Arc::new(AtomicUsize::new(0));
    let releases = Arc::new(AtomicUsize::new(0));
    let key = LoadKey::new("key", 8).unwrap();

    let running = executor.submit_ordered_with_hook(
        key.clone(),
        SourceReference::new(Some("running".to_owned()), false),
        counting_callback(&callbacks),
        Box::new(Hook(Arc::clone(&releases))),
    );
    assert_eq!(
        starts_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
        "running"
    );
    let pending = executor.submit_ordered_with_hook(
        key,
        SourceReference::new(Some("pending".to_owned()), false),
        counting_callback(&callbacks),
        Box::new(Hook(Arc::clone(&releases))),
    );

    assert!(pending.cancel());
    assert!(running.cancel());
    assert_eq!(pending.wait(), LoadHandleState::Cancelled);
    assert_eq!(running.wait(), LoadHandleState::Cancelled);
    executor.shutdown();

    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert_eq!(callbacks.load(Ordering::Relaxed), 0);
    assert_eq!(releases.load(Ordering::Relaxed), 2);
}

fn counting_callback(callbacks: &Arc<AtomicUsize>) -> Box<dyn SourceLoadResultHandler<String>> {
    let callbacks = Arc::clone(callbacks);
    callback(move |_| {
        callbacks.fetch_add(1, Ordering::Relaxed);
    })
}

#[test]
fn rejection_reports_one_failure_returns_a_completed_handle_and_releases_the_lease() {
    let (starts_tx, starts_rx) = mpsc::channel();
    let (_release_tx, release_rx) = mpsc::channel();
    let registry = controlled_registry(starts_tx, release_rx, Arc::new(AtomicUsize::new(0)));
    let mut executor = SourceLoadExecutor::<_, LoadKey>::new(
        registry,
        LoadExecutorLimits {
            workers: 1,
            maximum_pending: 1,
            maximum_channels: 1,
            maximum_per_channel: 1,
        },
    )
    .unwrap();
    let key = LoadKey::new("key", 8).unwrap();
    let accepted = executor.submit_ordered(
        key.clone(),
        SourceReference::new(Some("running".to_owned()), false),
        callback(|_| {}),
    );
    assert_eq!(
        starts_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
        "running"
    );
    let observed = Arc::new(Mutex::new(Vec::new()));
    let releases = Arc::new(AtomicUsize::new(0));
    let rejected = executor.submit_ordered_with_hook(
        key,
        SourceReference::new(Some("hit".to_owned()), false),
        result_collector(&observed),
        Box::new(Hook(Arc::clone(&releases))),
    );

    assert_eq!(rejected.wait(), LoadHandleState::Rejected);
    assert_eq!(releases.load(Ordering::Relaxed), 1);
    assert_eq!(observed.lock().unwrap().len(), 1);
    assert!(matches!(
        observed.lock().unwrap().first(),
        Some(SourceLoadResult::Failed(SourceLoadFailure::Rejected(_)))
    ));
    assert!(accepted.cancel());
    assert_eq!(accepted.wait(), LoadHandleState::Cancelled);
    executor.shutdown();
}

#[test]
fn shutdown_cancels_pending_and_running_work_and_rejects_future_submissions() {
    let (starts_tx, starts_rx) = mpsc::channel();
    let (_release_tx, release_rx) = mpsc::channel();
    let registry = controlled_registry(starts_tx, release_rx, Arc::new(AtomicUsize::new(0)));
    let mut executor = SourceLoadExecutor::<_, LoadKey>::new(
        registry,
        LoadExecutorLimits {
            workers: 1,
            maximum_pending: 2,
            maximum_channels: 0,
            maximum_per_channel: 0,
        },
    )
    .unwrap();
    let callbacks = Arc::new(AtomicUsize::new(0));
    let running = executor.submit(
        SourceReference::new(Some("running".to_owned()), false),
        counting_callback(&callbacks),
    );
    starts_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let pending = executor.submit(
        SourceReference::new(Some("pending".to_owned()), false),
        counting_callback(&callbacks),
    );

    executor.shutdown();
    assert_eq!(running.wait(), LoadHandleState::Cancelled);
    assert_eq!(pending.wait(), LoadHandleState::Cancelled);
    let rejected = executor.submit(
        SourceReference::new(Some("late".to_owned()), false),
        counting_callback(&callbacks),
    );
    assert_eq!(rejected.wait(), LoadHandleState::Rejected);
    assert_eq!(callbacks.load(Ordering::Relaxed), 1);
}
