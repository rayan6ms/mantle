use crate::{
    LoadError, LoadId, LoadScheduler, LoadedSourceItem, SourceCancellation, SourceReference,
    SourceRegistry, SourceRegistryError,
};
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Condvar, Mutex, RwLock, Weak};
use std::thread::{self, JoinHandle};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoadExecutorLimits {
    pub workers: usize,
    pub maximum_pending: usize,
    pub maximum_channels: usize,
    pub maximum_per_channel: usize,
}

impl Default for LoadExecutorLimits {
    fn default() -> Self {
        Self {
            workers: 10,
            maximum_pending: 5_000,
            maximum_channels: 5_000,
            maximum_per_channel: 5_000,
        }
    }
}

#[derive(Debug)]
pub enum LoadExecutorBuildError {
    InvalidLimits,
    WorkerSpawn(std::io::Error),
}

impl fmt::Display for LoadExecutorBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits => {
                formatter.write_str("load executor requires non-zero worker and pending limits")
            }
            Self::WorkerSpawn(error) => write!(formatter, "failed to start load worker: {error}"),
        }
    }
}

impl std::error::Error for LoadExecutorBuildError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceLoadFailure {
    Source(SourceRegistryError),
    Rejected(LoadError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceLoadResult<T> {
    Item(LoadedSourceItem<T>),
    NoMatches,
    Failed(SourceLoadFailure),
}

/// One-shot result callback. Implementations that enter another runtime must dispatch there.
pub trait SourceLoadResultHandler<T>: Send + 'static {
    fn finished(self: Box<Self>, result: SourceLoadResult<T>);
}

/// One-shot cleanup hook for boundary-owned ordering-key leases and similar resources.
///
/// This runs on the thread that observes the terminal transition. JVM implementations should
/// enqueue a token for release on an attached callback thread instead of dropping Java references
/// directly here.
pub trait LoadTerminalHook: Send + 'static {
    fn on_terminal(self: Box<Self>);
}

struct NoopTerminalHook;

impl LoadTerminalHook for NoopTerminalHook {
    fn on_terminal(self: Box<Self>) {}
}

/// Executes source selection synchronously and reports exactly one result.
pub fn dispatch_source_load<T: 'static>(
    registry: &SourceRegistry<T>,
    reference: &SourceReference,
    handler: Box<dyn SourceLoadResultHandler<T>>,
) {
    let result = catch_unwind(AssertUnwindSafe(|| registry.load(reference)))
        .unwrap_or(Err(SourceRegistryError::SourceFailure));
    let result = map_load_result(result);
    dispatch_handler(handler, result);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoadHandleState {
    Pending,
    Running,
    Complete,
    Cancelled,
    Rejected,
}

struct HandleStatus {
    state: Mutex<HandleStatusInner>,
    changed: Condvar,
}

struct HandleStatusInner {
    visible: LoadHandleState,
    cancellable: bool,
}

impl HandleStatus {
    fn new(state: LoadHandleState) -> Self {
        Self {
            state: Mutex::new(HandleStatusInner {
                visible: state,
                cancellable: matches!(state, LoadHandleState::Pending | LoadHandleState::Running),
            }),
            changed: Condvar::new(),
        }
    }

    fn get(&self) -> LoadHandleState {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .visible
    }

    fn mark_running(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.visible == LoadHandleState::Pending {
            state.visible = LoadHandleState::Running;
            self.changed.notify_all();
        }
    }

    fn request_cancel(&self) -> bool {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !state.cancellable {
            return false;
        }
        state.cancellable = false;
        state.visible = LoadHandleState::Cancelled;
        self.changed.notify_all();
        true
    }

    fn begin_completion(&self) -> bool {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !state.cancellable {
            return false;
        }
        state.cancellable = false;
        true
    }

    fn finish(&self, terminal: LoadHandleState) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if matches!(
            state.visible,
            LoadHandleState::Pending | LoadHandleState::Running
        ) {
            state.cancellable = false;
            state.visible = terminal;
            self.changed.notify_all();
        }
    }

    fn wait(&self) -> LoadHandleState {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        while matches!(
            state.visible,
            LoadHandleState::Pending | LoadHandleState::Running
        ) {
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        state.visible
    }
}

type CancelAction = dyn Fn(LoadId) -> bool + Send + Sync;

/// Cancellation and completion view returned for every accepted or rejected asynchronous load.
pub struct SourceLoadHandle {
    id: Option<LoadId>,
    cancellation: SourceCancellation,
    status: Arc<HandleStatus>,
    cancel_action: Option<Arc<CancelAction>>,
}

impl SourceLoadHandle {
    #[must_use]
    pub const fn id(&self) -> Option<LoadId> {
        self.id
    }

    #[must_use]
    pub fn state(&self) -> LoadHandleState {
        self.status.get()
    }

    /// Cooperatively cancels queued or running work.
    #[must_use]
    pub fn cancel(&self) -> bool {
        let Some(id) = self.id else {
            return false;
        };
        if !self.status.request_cancel() {
            return false;
        }
        self.cancellation.cancel();
        if let Some(cancel) = &self.cancel_action {
            let _ = cancel(id);
        }
        true
    }

    /// Waits until the load reaches a terminal state.
    #[must_use]
    pub fn wait(&self) -> LoadHandleState {
        self.status.wait()
    }
}

struct LoadJob<T> {
    reference: SourceReference,
    handler: Box<dyn SourceLoadResultHandler<T>>,
    cancellation: SourceCancellation,
    status: Arc<HandleStatus>,
    terminal_hook: Box<dyn LoadTerminalHook>,
}

impl<T> LoadJob<T> {
    fn cancel_queued(self) {
        run_terminal_hook(self.terminal_hook);
        self.status.finish(LoadHandleState::Cancelled);
    }
}

struct RunningLoad {
    cancellation: SourceCancellation,
    status: Arc<HandleStatus>,
}

struct ExecutorState<T, K>
where
    K: Clone + Eq + Hash,
{
    scheduler: LoadScheduler<LoadJob<T>, K>,
    running: HashMap<LoadId, RunningLoad>,
    closed: bool,
}

struct Shared<T, K>
where
    K: Clone + Eq + Hash,
{
    state: Mutex<ExecutorState<T, K>>,
    available: Condvar,
}

/// Fixed-size bounded source loader with FIFO equality-key channels.
pub struct SourceLoadExecutor<T, K>
where
    K: Clone + Eq + Hash,
{
    registry: Arc<RwLock<SourceRegistry<T>>>,
    shared: Arc<Shared<T, K>>,
    workers: Vec<JoinHandle<()>>,
}

impl<T, K> SourceLoadExecutor<T, K>
where
    T: Send + 'static,
    K: Clone + Eq + Hash + Send + 'static,
{
    /// Appends a source registration visible to subsequent loads.
    ///
    /// # Errors
    ///
    /// Returns the registry's lifecycle, validation, or capacity error.
    pub fn register_source(
        &self,
        manager: Box<dyn crate::SourceManager<T>>,
    ) -> Result<crate::SourceRegistrationId, SourceRegistryError> {
        self.registry
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .register(manager)
    }

    /// Runs source selection on the calling thread against the current registration snapshot.
    ///
    /// # Errors
    ///
    /// Returns the registry's lifecycle, validation, or source error.
    pub fn load_sync(
        &self,
        reference: &SourceReference,
    ) -> Result<Option<LoadedSourceItem<T>>, SourceRegistryError> {
        self.registry_snapshot().load(reference)
    }

    /// Encodes details with the registration that produced the item.
    ///
    /// # Errors
    ///
    /// Returns the registry's ownership, source, encodability, or size error.
    pub fn encode_details(
        &self,
        item: &LoadedSourceItem<T>,
    ) -> Result<crate::SourceDetails, SourceRegistryError> {
        let registry = self.registry_snapshot();
        registry.encode_details(item)
    }

    /// Decodes details with common track metadata through the first matching source registration.
    ///
    /// # Errors
    ///
    /// Returns the registry's source, name, or payload-size error.
    pub fn decode_details_with_info(
        &self,
        info: &crate::TrackInfo,
        details: &crate::SourceDetails,
    ) -> Result<Option<LoadedSourceItem<T>>, SourceRegistryError> {
        let registry = self.registry_snapshot();
        registry.decode_details_with_info(info, details)
    }

    /// Starts a fixed number of named loader workers.
    ///
    /// # Errors
    ///
    /// Returns an error for unusable limits or when an operating-system worker cannot be started.
    pub fn new(
        registry: SourceRegistry<T>,
        limits: LoadExecutorLimits,
    ) -> Result<Self, LoadExecutorBuildError> {
        if limits.workers == 0 || limits.maximum_pending == 0 {
            return Err(LoadExecutorBuildError::InvalidLimits);
        }
        let registry = Arc::new(RwLock::new(registry));
        let shared = Arc::new(Shared {
            state: Mutex::new(ExecutorState {
                scheduler: LoadScheduler::new_for_keys(
                    limits.maximum_pending,
                    limits.maximum_channels,
                    limits.maximum_per_channel,
                ),
                running: HashMap::new(),
                closed: false,
            }),
            available: Condvar::new(),
        });
        let mut workers = Vec::with_capacity(limits.workers);
        for index in 0..limits.workers {
            let worker_registry = Arc::clone(&registry);
            let worker_shared = Arc::clone(&shared);
            match thread::Builder::new()
                .name(format!("mantle-info-loader-{}", index + 1))
                .spawn(move || worker_loop(&worker_registry, &worker_shared))
            {
                Ok(worker) => workers.push(worker),
                Err(error) => {
                    close_shared(&shared);
                    for worker in workers {
                        let _ = worker.join();
                    }
                    return Err(LoadExecutorBuildError::WorkerSpawn(error));
                }
            }
        }
        Ok(Self {
            registry,
            shared,
            workers,
        })
    }

    #[must_use]
    pub fn submit(
        &self,
        reference: SourceReference,
        handler: Box<dyn SourceLoadResultHandler<T>>,
    ) -> SourceLoadHandle {
        self.submit_inner(None, reference, handler, Box::new(NoopTerminalHook))
    }

    #[must_use]
    pub fn submit_ordered(
        &self,
        key: K,
        reference: SourceReference,
        handler: Box<dyn SourceLoadResultHandler<T>>,
    ) -> SourceLoadHandle {
        self.submit_inner(Some(key), reference, handler, Box::new(NoopTerminalHook))
    }

    #[must_use]
    pub fn submit_ordered_with_hook(
        &self,
        key: K,
        reference: SourceReference,
        handler: Box<dyn SourceLoadResultHandler<T>>,
        terminal_hook: Box<dyn LoadTerminalHook>,
    ) -> SourceLoadHandle {
        self.submit_inner(Some(key), reference, handler, terminal_hook)
    }

    fn submit_inner(
        &self,
        key: Option<K>,
        reference: SourceReference,
        handler: Box<dyn SourceLoadResultHandler<T>>,
        terminal_hook: Box<dyn LoadTerminalHook>,
    ) -> SourceLoadHandle {
        let cancellation = SourceCancellation::new();
        let status = Arc::new(HandleStatus::new(LoadHandleState::Pending));
        let job = LoadJob {
            reference,
            handler,
            cancellation: cancellation.clone(),
            status: Arc::clone(&status),
            terminal_hook,
        };
        let submission = {
            let mut state = self
                .shared
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if state.closed {
                Err((LoadError::Shutdown, job))
            } else {
                state.scheduler.submit_recover(key, job)
            }
        };
        match submission {
            Ok(id) => {
                self.shared.available.notify_one();
                let weak = Arc::downgrade(&self.shared);
                SourceLoadHandle {
                    id: Some(id),
                    cancellation,
                    status,
                    cancel_action: Some(Arc::new(move |id| cancel_shared(&weak, id))),
                }
            }
            Err((error, job)) => rejected_handle(error, job),
        }
    }

    /// Cancels queued/running work, rejects new submissions, shuts sources down, and joins workers.
    pub fn shutdown(&mut self) {
        let pending = close_shared(&self.shared);
        shutdown_registry(&self.registry);
        for job in pending {
            job.cancel_queued();
        }
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

impl<T, K> Drop for SourceLoadExecutor<T, K>
where
    K: Clone + Eq + Hash,
{
    fn drop(&mut self) {
        let pending = close_shared(&self.shared);
        shutdown_registry(&self.registry);
        for job in pending {
            job.cancel_queued();
        }
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

impl<T, K> SourceLoadExecutor<T, K>
where
    K: Clone + Eq + Hash,
{
    fn registry_snapshot(&self) -> SourceRegistry<T> {
        self.registry
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .load_snapshot()
    }
}

fn shutdown_registry<T>(registry: &RwLock<SourceRegistry<T>>) {
    let snapshot = registry
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .begin_shutdown();
    if let Some(snapshot) = snapshot {
        snapshot.shutdown_registrations();
    }
}

fn worker_loop<T, K>(registry: &RwLock<SourceRegistry<T>>, shared: &Shared<T, K>)
where
    T: Send + 'static,
    K: Clone + Eq + Hash + Send + 'static,
{
    loop {
        let scheduled = {
            let mut state = shared
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            loop {
                if let Some(scheduled) = state.scheduler.take_ready() {
                    scheduled.value.status.mark_running();
                    state.running.insert(
                        scheduled.id,
                        RunningLoad {
                            cancellation: scheduled.value.cancellation.clone(),
                            status: Arc::clone(&scheduled.value.status),
                        },
                    );
                    break Some(scheduled);
                }
                if state.closed {
                    break None;
                }
                state = shared
                    .available
                    .wait(state)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        };
        let Some(scheduled) = scheduled else {
            return;
        };
        let id = scheduled.id;
        let job = scheduled.value;
        let cancelled_before_selection = job.cancellation.is_cancelled();
        let result = (!cancelled_before_selection).then(|| {
            catch_unwind(AssertUnwindSafe(|| {
                let registry = registry
                    .read()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .load_snapshot();
                registry.load_with_cancellation(&job.reference, &job.cancellation)
            }))
            .unwrap_or(Err(SourceRegistryError::SourceFailure))
        });
        let deliver = !job.cancellation.is_cancelled() && job.status.begin_completion();
        if deliver && let Some(result) = result {
            dispatch_handler(job.handler, map_load_result(result));
        }
        {
            let mut state = shared
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.running.remove(&id);
            let _ = state.scheduler.complete(id);
        }
        run_terminal_hook(job.terminal_hook);
        if deliver {
            job.status.finish(LoadHandleState::Complete);
        } else {
            job.status.finish(LoadHandleState::Cancelled);
        }
        shared.available.notify_all();
    }
}

fn map_load_result<T>(
    result: Result<Option<LoadedSourceItem<T>>, SourceRegistryError>,
) -> SourceLoadResult<T> {
    match result {
        Ok(Some(item)) => SourceLoadResult::Item(item),
        Ok(None) => SourceLoadResult::NoMatches,
        Err(error) => SourceLoadResult::Failed(SourceLoadFailure::Source(error)),
    }
}

fn dispatch_handler<T: 'static>(
    handler: Box<dyn SourceLoadResultHandler<T>>,
    result: SourceLoadResult<T>,
) {
    let _ = catch_unwind(AssertUnwindSafe(|| handler.finished(result)));
}

fn run_terminal_hook(hook: Box<dyn LoadTerminalHook>) {
    let _ = catch_unwind(AssertUnwindSafe(|| hook.on_terminal()));
}

fn rejected_handle<T: 'static>(error: LoadError, job: LoadJob<T>) -> SourceLoadHandle {
    dispatch_handler(
        job.handler,
        SourceLoadResult::Failed(SourceLoadFailure::Rejected(error)),
    );
    run_terminal_hook(job.terminal_hook);
    job.status.finish(LoadHandleState::Rejected);
    SourceLoadHandle {
        id: None,
        cancellation: job.cancellation,
        status: job.status,
        cancel_action: None,
    }
}

fn cancel_shared<T, K>(shared: &Weak<Shared<T, K>>, id: LoadId) -> bool
where
    K: Clone + Eq + Hash,
{
    let Some(shared) = shared.upgrade() else {
        return false;
    };
    let cancelled = {
        let mut state = shared
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.scheduler.cancel_take(id).ok().flatten()
    };
    let Some(cancelled) = cancelled else {
        return false;
    };
    if let Some(job) = cancelled.value {
        job.cancel_queued();
    }
    shared.available.notify_all();
    true
}

fn close_shared<T, K>(shared: &Shared<T, K>) -> Vec<LoadJob<T>>
where
    K: Clone + Eq + Hash,
{
    let (pending, running) = {
        let mut state = shared
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.closed {
            return Vec::new();
        }
        state.closed = true;
        let running = state
            .running
            .values()
            .map(|running| (running.cancellation.clone(), Arc::clone(&running.status)))
            .collect::<Vec<_>>();
        let pending = state.scheduler.shutdown_take_pending();
        (pending, running)
    };
    for (cancellation, status) in running {
        let _ = status.request_cancel();
        cancellation.cancel();
    }
    shared.available.notify_all();
    pending
}
