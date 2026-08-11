mod proxy;
mod registry;

#[cfg(feature = "gate-a-direct-attachment")]
use std::ffi::c_void;
#[cfg(feature = "gate-a-direct-attachment")]
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use jni::EnvUnowned;
use jni::errors::ThrowRuntimeExAndDefault;
use jni::objects::{JClass, JObject, JObjectArray, JString, JValue};
#[cfg(feature = "gate-a-direct-attachment")]
use jni::sys;
use jni::{jni_sig, jni_str};
use registry::{Handle, HandleKind, Registry};

const ABI_VERSION: i32 = 1;
const CAPABILITIES: i64 = 0b111;
const BUILD_ID: &str = "mantle-gate-a-1";

static CALLBACK_EXCEPTIONS: AtomicUsize = AtomicUsize::new(0);

fn registry() -> &'static Mutex<Registry> {
    static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(Registry::default()))
}

fn with_registry<T>(operation: impl FnOnce(&mut Registry) -> T) -> T {
    let mut registry = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    operation(&mut registry)
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
        Ok(with_registry(|registry| registry.insert(kind).as_jlong()))
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
    env.with_env(|_| {
        if let Ok(handle) = Handle::from_jlong(raw_handle) {
            with_registry(|registry| {
                registry.release(handle);
            });
        }
        Ok::<_, jni::errors::Error>(())
    })
    .resolve::<ThrowRuntimeExAndDefault>();
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
    env.with_env(|env| {
        let pending = identifier.try_to_string(env)? == "gate:pending";
        let identifier = JObject::from(identifier);
        let track = proxy::create(env, 3, &identifier)?;
        let future = env.new_object(
            jni_str!("java/util/concurrent/CompletableFuture"),
            jni_sig!("()V"),
            &[],
        )?;
        let _ = env.call_method(
            &result_handler,
            jni_str!("trackLoaded"),
            jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;)V"),
            &[JValue::Object(&track)],
        )?;
        if !pending {
            let _ = env.call_method(
                &future,
                jni_str!("complete"),
                jni_sig!("(Ljava/lang/Object;)Z"),
                &[JValue::Object(&JObject::null())],
            )?;
        }
        Ok::<_, jni::errors::Error>(future)
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}
