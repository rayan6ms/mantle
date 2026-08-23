mod load_bridge;
mod ordering_key;
mod playback_bridge;
mod proxy;
mod registry;

pub use ordering_key::{JvmOrderingKeyReleaseQueue, JvmOrderingKeyTable};

#[cfg(feature = "gate-a-direct-attachment")]
use std::ffi::c_void;
#[cfg(feature = "gate-a-direct-attachment")]
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use jni::EnvUnowned;
use jni::errors::ThrowRuntimeExAndDefault;
use jni::objects::{JByteArray, JClass, JObject, JObjectArray, JString};
#[cfg(feature = "gate-a-direct-attachment")]
use jni::sys;
use jni::{jni_sig, jni_str};
use mantle_core::{
    Engine, Frame, ManagerId, ResourceLimits, SerializationLimits, SystemClock, TrackInfo,
    decode_synthetic_track_details, encode_synthetic_track_details,
};
use registry::{CoreObject, Handle, HandleKind, Registry};

const ABI_VERSION: i32 = 1;
const CAPABILITIES: i64 = 0b111;
const BUILD_ID: &str = "mantle-gate-a-1";

static CALLBACK_EXCEPTIONS: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn record_callback_exception() {
    CALLBACK_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
}

fn registry() -> &'static Mutex<Registry> {
    static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(Registry::default()))
}

fn engine() -> &'static Mutex<Engine<SystemClock>> {
    static ENGINE: OnceLock<Mutex<Engine<SystemClock>>> = OnceLock::new();
    ENGINE.get_or_init(|| Mutex::new(Engine::new(SystemClock::new(), ResourceLimits::default())))
}

fn default_manager() -> &'static Mutex<Option<ManagerId>> {
    static DEFAULT_MANAGER: OnceLock<Mutex<Option<ManagerId>>> = OnceLock::new();
    DEFAULT_MANAGER.get_or_init(|| Mutex::new(None))
}

fn with_registry<T>(operation: impl FnOnce(&mut Registry) -> T) -> T {
    let mut registry = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    operation(&mut registry)
}

pub(crate) fn with_engine<T>(operation: impl FnOnce(&mut Engine<SystemClock>) -> T) -> T {
    let mut engine = engine()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    operation(&mut engine)
}

fn create_core_object(
    kind: HandleKind,
    identifier: Option<&str>,
) -> Result<Option<CoreObject>, &'static str> {
    match kind {
        HandleKind::Manager => {
            let manager =
                with_engine(Engine::create_manager).map_err(|_| "could not create core manager")?;
            *default_manager()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(manager);
            Ok(Some(CoreObject::Manager(manager)))
        }
        HandleKind::Player => {
            let manager = default_manager()
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .ok_or("player requires a live manager")?;
            with_engine(|engine| engine.create_player(manager))
                .map(CoreObject::Player)
                .map(Some)
                .map_err(|_| "could not create core player")
        }
        HandleKind::Track => {
            let identifier = identifier.unwrap_or("gate:track");
            let info = TrackInfo {
                title: "Synthetic title".into(),
                author: "Synthetic author".into(),
                duration: Duration::from_secs(1),
                identifier: identifier.into(),
                is_stream: false,
                uri: Some(format!("oracle://{identifier}")),
                artwork_url: Some("oracle://artwork".into()),
                isrc: Some("ORACLE000001".into()),
            };
            with_engine(|engine| {
                engine.create_track(info, [Frame::synthetic(Duration::ZERO, [1_u8, 2, 3, 4])])
            })
            .map(CoreObject::Track)
            .map(Some)
            .map_err(|_| "could not create core track")
        }
        HandleKind::Frame | HandleKind::Probe => Ok(None),
    }
}

pub(crate) fn core_for_handle(
    raw_handle: i64,
    kind: HandleKind,
) -> jni::errors::Result<CoreObject> {
    let handle = Handle::from_jlong(raw_handle)
        .map_err(|_| jni::errors::Error::NullPtr("invalid native handle"))?;
    with_registry(|registry| registry.core(handle, kind))
        .map_err(|_| jni::errors::Error::NullPtr("native handle has no core object"))
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_abiVersion<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
) -> i32 {
    env.with_env(|_| Ok::<_, jni::errors::Error>(ABI_VERSION))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_buildId<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
) -> JString<'local> {
    env.with_env(|env| env.new_string(BUILD_ID))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_capabilities<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
) -> i64 {
    env.with_env(|_| Ok::<_, jni::errors::Error>(CAPABILITIES))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_createHandle<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    kind: i32,
) -> i64 {
    env.with_env(|_| {
        let kind = match kind {
            1 => HandleKind::Manager,
            2 => HandleKind::Player,
            3 => HandleKind::Track,
            4 => HandleKind::Frame,
            5 => HandleKind::Probe,
            _ => return Err(jni::errors::Error::NullPtr("unknown native handle kind")),
        };
        let core = create_core_object(kind, None).map_err(jni::errors::Error::NullPtr)?;
        Ok(with_registry(|registry| {
            registry.insert(kind, core).as_jlong()
        }))
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_release<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    raw_handle: i64,
) {
    env.with_env(|env| {
        if let Ok(handle) = Handle::from_jlong(raw_handle) {
            let core = with_registry(|registry| registry.release(handle));
            if let Some(core) = core {
                if matches!(core, CoreObject::Manager(_)) {
                    load_bridge::shutdown(env);
                }
                with_engine(|engine| match core {
                    CoreObject::Manager(manager) => {
                        let _ = engine.release_manager(manager);
                        let mut default = default_manager()
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        if *default == Some(manager) {
                            *default = None;
                        }
                    }
                    CoreObject::Player(player) => {
                        let _ = engine.release_player(player);
                    }
                    CoreObject::Track(track) => {
                        let _ = engine.release_track(track);
                    }
                });
            }
        }
        Ok::<_, jni::errors::Error>(())
    })
    .resolve::<ThrowRuntimeExAndDefault>();
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_createCoreHandle<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    kind: i32,
    identifier: JString<'local>,
) -> i64 {
    env.with_env(|env| {
        let kind = match kind {
            2 => HandleKind::Player,
            3 => HandleKind::Track,
            4 => HandleKind::Frame,
            _ => return Err(jni::errors::Error::NullPtr("unknown proxy handle kind")),
        };
        let identifier = if identifier.is_null() {
            None
        } else {
            Some(identifier.try_to_string(env)?)
        };
        let core =
            create_core_object(kind, identifier.as_deref()).map_err(jni::errors::Error::NullPtr)?;
        Ok(with_registry(|registry| {
            registry.insert(kind, core).as_jlong()
        }))
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_liveHandles<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
) -> i32 {
    env.with_env(|_| {
        i32::try_from(with_registry(|registry| registry.live()))
            .map_err(|_| jni::errors::Error::NullPtr("native handle count exceeds i32"))
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_validateHandle<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    raw_handle: i64,
    kind: i32,
) -> bool {
    env.with_env(|_| {
        let handle = Handle::from_jlong(raw_handle)
            .map_err(|_| jni::errors::Error::NullPtr("invalid native handle"))?;
        let kind = match kind {
            1 => HandleKind::Manager,
            2 => HandleKind::Player,
            3 => HandleKind::Track,
            4 => HandleKind::Frame,
            5 => HandleKind::Probe,
            _ => return Err(jni::errors::Error::NullPtr("unknown native handle kind")),
        };
        with_registry(|registry| registry.validate(handle, kind))
            .map(|()| true)
            .map_err(|_| jni::errors::Error::NullPtr("native handle validation failed"))
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_identity<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    object: JObject<'local>,
) -> JObject<'local> {
    env.with_env(|_| Ok::<_, jni::errors::Error>(object))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_dispatchOnCurrentThread<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    callback: JObject<'local>,
    iterations: i32,
) {
    env.with_env(|env| {
        if iterations < 0 {
            return Err(jni::errors::Error::NullPtr(
                "callback iteration count must be non-negative",
            ));
        }
        let (sender, receiver) = std::sync::mpsc::sync_channel::<()>(1_024);
        let producer = std::thread::Builder::new()
            .name("mantle-callback-producer-spike".to_owned())
            .spawn(move || {
                for _ in 0..iterations {
                    if sender.send(()).is_err() {
                        break;
                    }
                }
            })
            .map_err(|_| jni::errors::Error::NullPtr("could not spawn callback producer"))?;
        for _ in 0..iterations {
            receiver
                .recv()
                .map_err(|_| jni::errors::Error::NullPtr("callback producer stopped early"))?;
            if env
                .call_method(&callback, jni_str!("run"), jni_sig!("()V"), &[])
                .is_err()
            {
                CALLBACK_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
            }
        }
        producer
            .join()
            .map_err(|_| jni::errors::Error::NullPtr("callback producer panicked"))?;
        Ok::<_, jni::errors::Error>(())
    })
    .resolve::<ThrowRuntimeExAndDefault>();
}

#[allow(
    unsafe_code,
    reason = "Gate A directly evaluates AttachCurrentThreadAsDaemon, which jni-rs intentionally does not wrap"
)]
#[cfg(feature = "gate-a-direct-attachment")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_dispatchOnNativeDaemon<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    callback: JObject<'local>,
    iterations: i32,
) -> bool {
    env.with_env(|env| {
        if iterations < 0 {
            return Err(jni::errors::Error::NullPtr(
                "callback iteration count must be non-negative",
            ));
        }

        let vm = env.get_java_vm()?.get_raw() as usize;
        let class = env.get_object_class(&callback)?;
        let run = env
            .get_method_id(class, jni_str!("run"), jni_sig!("()V"))?
            .into_raw() as usize;
        let raw_env = env.get_raw();
        // SAFETY: `raw_env` is valid for this Java-originating call. The new global
        // reference is the only JVM object moved to the worker and is deleted before
        // that worker detaches. The captured method ID remains valid while the global
        // callback (and therefore its defining class) is alive.
        let global = unsafe {
            let functions = &**raw_env;
            (functions.v1_1.NewGlobalRef)(raw_env, callback.as_raw())
        } as usize;
        if global == 0 {
            return Err(jni::errors::Error::NullPtr(
                "could not create callback global reference",
            ));
        }

        std::thread::Builder::new()
            .name("mantle-direct-daemon-spike".to_owned())
            .spawn(move || {
                // SAFETY: this is the deliberately narrow raw-JNI Gate A spike. All
                // raw values originate from the same live VM. No RAII JNI value is
                // created on the daemon thread, the global is deleted before detach,
                // and no JNI access occurs after detach.
                unsafe {
                    direct_daemon_worker(vm, global, run, iterations);
                }
            })
            .map(|_| true)
            .map_err(|_| jni::errors::Error::NullPtr("could not spawn callback worker"))
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(
    unsafe_code,
    reason = "raw JNI is confined to the daemon-attachment design spike"
)]
#[cfg(feature = "gate-a-direct-attachment")]
unsafe fn direct_daemon_worker(vm: usize, callback: usize, run: usize, iterations: i32) {
    let vm = vm as *mut sys::JavaVM;
    let callback = callback as sys::jobject;
    let run = run as sys::jmethodID;
    let mut raw_env: *mut c_void = ptr::null_mut();
    // SAFETY: `vm` was obtained from JNIEnv in the initiating native call and the
    // process still owns that VM for the duration of this short-lived spike worker.
    let attached = unsafe {
        let functions = &**vm;
        (functions.v1_4.AttachCurrentThreadAsDaemon)(vm, &raw mut raw_env, ptr::null_mut())
    };
    if attached != sys::JNI_OK {
        CALLBACK_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
        return;
    }
    let env = raw_env.cast::<sys::JNIEnv>();
    // SAFETY: successful attachment returned a valid JNIEnv for this thread.
    let functions = unsafe { &**env };
    for _ in 0..iterations {
        // SAFETY: callback is a live global reference and run is its `Runnable.run` ID.
        unsafe {
            (functions.v1_1.CallVoidMethodA)(env, callback, run, ptr::null());
        }
        // Clear callback exceptions so the loop and cleanup remain well-defined.
        if unsafe { (functions.v1_2.ExceptionCheck)(env) } != sys::JNI_FALSE {
            CALLBACK_EXCEPTIONS.fetch_add(1, Ordering::Relaxed);
            unsafe {
                (functions.v1_1.ExceptionClear)(env);
            }
        }
    }
    unsafe {
        (functions.v1_1.DeleteGlobalRef)(env, callback);
        let vm_functions = &**vm;
        let _ = (vm_functions.v1_1.DetachCurrentThread)(vm);
    }
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_callbackExceptions<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
) -> i32 {
    env.with_env(|_| {
        i32::try_from(CALLBACK_EXCEPTIONS.load(Ordering::Relaxed))
            .map_err(|_| jni::errors::Error::NullPtr("callback exception count exceeds i32"))
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_resetCallbackExceptions<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
) {
    env.with_env(|_| {
        CALLBACK_EXCEPTIONS.store(0, Ordering::Relaxed);
        Ok::<_, jni::errors::Error>(())
    })
    .resolve::<ThrowRuntimeExAndDefault>();
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_createProxy<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    kind: i32,
    identifier: JString<'local>,
) -> JObject<'local> {
    env.with_env(|env| {
        let identifier = JObject::from(identifier);
        proxy::create(env, kind, &identifier)
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_NativeInvocationHandler_invoke<'local>(
    mut env: EnvUnowned<'local>,
    handler: JObject<'local>,
    object: JObject<'local>,
    method: JObject<'local>,
    args: JObjectArray<'local, JObject<'local>>,
) -> JObject<'local> {
    env.with_env(|env| proxy::invoke(env, &handler, &object, &method, &args))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_loadItem<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    identifier: JString<'local>,
    result_handler: JObject<'local>,
) -> JObject<'local> {
    env.with_env(|env| load_bridge::submit(env, &identifier, &result_handler, None))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_loadItemOrdered<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    ordering_key: JObject<'local>,
    identifier: JString<'local>,
    result_handler: JObject<'local>,
) -> JObject<'local> {
    env.with_env(|env| load_bridge::submit(env, &identifier, &result_handler, Some(&ordering_key)))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_loadItemReference<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    reference: JObject<'local>,
    result_handler: JObject<'local>,
) -> JObject<'local> {
    env.with_env(|env| load_bridge::submit_java_reference(env, &reference, &result_handler, None))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_loadItemOrderedReference<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    ordering_key: JObject<'local>,
    reference: JObject<'local>,
    result_handler: JObject<'local>,
) -> JObject<'local> {
    env.with_env(|env| {
        load_bridge::submit_java_reference(env, &reference, &result_handler, Some(&ordering_key))
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_loadItemSync<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    reference: JObject<'local>,
) -> JObject<'local> {
    env.with_env(|env| load_bridge::load_sync(env, &reference))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_loadItemSyncHandled<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    reference: JObject<'local>,
    result_handler: JObject<'local>,
) {
    env.with_env(|env| load_bridge::load_sync_handled(env, &reference, &result_handler))
        .resolve::<ThrowRuntimeExAndDefault>();
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_loadNicoItem<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    source: JObject<'local>,
    reference: JObject<'local>,
) -> JObject<'local> {
    env.with_env(|env| load_bridge::load_nico_item(env, &source, &reference))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_loadTwitchItem<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    source: JObject<'local>,
    reference: JObject<'local>,
) -> JObject<'local> {
    env.with_env(|env| load_bridge::load_twitch_item(env, &source, &reference))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_loadVimeoItem<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    source: JObject<'local>,
    reference: JObject<'local>,
) -> JObject<'local> {
    env.with_env(|env| load_bridge::load_vimeo_item(env, &source, &reference))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_processNicoTrack<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    track: JObject<'local>,
    executor: JObject<'local>,
) {
    env.with_env(|env| playback_bridge::process_nico_track(env, &track, &executor))
        .resolve::<ThrowRuntimeExAndDefault>();
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_processSoundCloudTrack<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    track: JObject<'local>,
    executor: JObject<'local>,
) {
    env.with_env(|env| playback_bridge::process_sound_cloud_track(env, &track, &executor))
        .resolve::<ThrowRuntimeExAndDefault>();
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_processTwitchTrack<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    track: JObject<'local>,
    executor: JObject<'local>,
) {
    env.with_env(|env| playback_bridge::process_twitch_track(env, &track, &executor))
        .resolve::<ThrowRuntimeExAndDefault>();
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_processVimeoTrack<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    track: JObject<'local>,
    executor: JObject<'local>,
) {
    env.with_env(|env| playback_bridge::process_vimeo_track(env, &track, &executor))
        .resolve::<ThrowRuntimeExAndDefault>();
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_cancelLoad<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    load_id: i64,
    _may_interrupt: bool,
) -> bool {
    env.with_env(|_| Ok::<_, jni::errors::Error>(load_bridge::cancel(load_id)))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_dispatchLoad<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    load_id: i64,
) {
    env.with_env(|env| load_bridge::dispatch(env, load_id))
        .resolve::<ThrowRuntimeExAndDefault>();
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_orderingKeyCount<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
) -> i32 {
    env.with_env(|_| {
        i32::try_from(load_bridge::ordering_key_count())
            .map_err(|_| jni::errors::Error::NullPtr("ordering-key count overflow"))
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_registerSourceManager<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    player_manager: JObject<'local>,
    source: JObject<'local>,
) {
    env.with_env(|env| load_bridge::register(env, &player_manager, &source))
        .resolve::<ThrowRuntimeExAndDefault>();
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_sourceManager<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    player_manager: JObject<'local>,
    requested_class: JClass<'local>,
) -> JObject<'local> {
    env.with_env(|env| load_bridge::source_manager(env, &player_manager, &requested_class))
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_encodeTrackDetails<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    track: JObject<'local>,
) -> JByteArray<'local> {
    env.with_env(|env| {
        if let Some(bytes) = load_bridge::encode_track_details(env, &track)? {
            return env.byte_array_from_slice(&bytes);
        }
        let _ = proxy::track_id_from_proxy(env, &track)?;
        let bytes = encode_synthetic_track_details(SerializationLimits::default())
            .map_err(|_| jni::errors::Error::NullPtr("could not encode synthetic track details"))?;
        env.byte_array_from_slice(&bytes)
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_trackedSourceItemCount<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
) -> i32 {
    env.with_env(|env| {
        i32::try_from(load_bridge::tracked_source_item_count(env)?)
            .map_err(|_| jni::errors::Error::NullPtr("tracked source-item count exceeds i32"))
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

#[allow(unsafe_code, reason = "JNI requires stable exported symbol names")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_mantle_internal_MantleNative_decodeTrackDetails<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    info: JObject<'local>,
    bytes: JByteArray<'local>,
) -> JObject<'local> {
    env.with_env(|env| {
        let limits = SerializationLimits::default();
        if bytes.len(env)? > limits.message_bytes {
            return Err(jni::errors::Error::NullPtr(
                "synthetic track details exceed their byte limit",
            ));
        }
        let bytes = env.convert_byte_array(&bytes)?;
        let info = track_info_from_java(env, &info)?;
        if let Some(track) = load_bridge::decode_track_details(env, &info, &bytes)? {
            return Ok(track);
        }
        decode_synthetic_track_details(&bytes, limits)
            .map_err(|_| jni::errors::Error::NullPtr("could not decode synthetic track details"))?;
        let identifier = env.new_string(&info.identifier)?;
        let track = proxy::create(env, 3, identifier.as_ref())?;
        let track_id = proxy::track_id_from_proxy(env, &track)?;
        with_engine(|engine| engine.replace_track_info(track_id, info))
            .map_err(|_| jni::errors::Error::NullPtr("could not apply decoded track metadata"))?;
        Ok::<_, jni::errors::Error>(track)
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

pub(crate) fn track_info_from_java(
    env: &mut jni::Env<'_>,
    info: &JObject<'_>,
) -> jni::errors::Result<TrackInfo> {
    let duration = env
        .get_field(info, jni_str!("length"), jni_sig!("J"))?
        .j()?;
    Ok(TrackInfo {
        title: required_string_field(env, info, "title")?,
        author: required_string_field(env, info, "author")?,
        duration: Duration::from_millis(
            u64::try_from(duration)
                .map_err(|_| jni::errors::Error::NullPtr("negative track duration"))?,
        ),
        identifier: required_string_field(env, info, "identifier")?,
        is_stream: env
            .get_field(info, jni_str!("isStream"), jni_sig!("Z"))?
            .z()?,
        uri: optional_string_field(env, info, "uri")?,
        artwork_url: optional_string_field(env, info, "artworkUrl")?,
        isrc: optional_string_field(env, info, "isrc")?,
    })
}

fn required_string_field(
    env: &mut jni::Env<'_>,
    object: &JObject<'_>,
    name: &str,
) -> jni::errors::Result<String> {
    let value = env
        .get_field(
            object,
            jni::strings::JNIString::from(name),
            jni_sig!("Ljava/lang/String;"),
        )?
        .l()?;
    validate_java_string_length(env, &value)?;
    JString::cast_local(env, value)?.try_to_string(env)
}

fn optional_string_field(
    env: &mut jni::Env<'_>,
    object: &JObject<'_>,
    name: &str,
) -> jni::errors::Result<Option<String>> {
    let value = env
        .get_field(
            object,
            jni::strings::JNIString::from(name),
            jni_sig!("Ljava/lang/String;"),
        )?
        .l()?;
    if value.is_null() {
        Ok(None)
    } else {
        validate_java_string_length(env, &value)?;
        JString::cast_local(env, value)?
            .try_to_string(env)
            .map(Some)
    }
}

fn validate_java_string_length(
    env: &mut jni::Env<'_>,
    value: &JObject<'_>,
) -> jni::errors::Result<()> {
    let length = env
        .call_method(value, jni_str!("length"), jni_sig!("()I"), &[])?
        .i()?;
    let maximum = ResourceLimits::default().metadata_bytes;
    if usize::try_from(length).map_or(true, |length| length > maximum) {
        Err(jni::errors::Error::NullPtr(
            "track metadata exceeds its string limit",
        ))
    } else {
        Ok(())
    }
}
