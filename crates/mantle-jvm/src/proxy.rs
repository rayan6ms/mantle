use jni::objects::{JObject, JObjectArray, JString, JValue};
use jni::{Env, jni_sig, jni_str};

const HANDLER: &jni::strings::JNIStr = jni_str!("dev/mantle/internal/NativeInvocationHandler");

pub(crate) fn create<'local>(
    env: &mut Env<'local>,
    kind: i32,
    identifier: &JObject<'local>,
) -> jni::errors::Result<JObject<'local>> {
    let interface_name = match kind {
        2 => jni_str!("com/sedmelluq/discord/lavaplayer/player/AudioPlayer"),
        3 => jni_str!("com/sedmelluq/discord/lavaplayer/track/AudioTrack"),
        4 => jni_str!("com/sedmelluq/discord/lavaplayer/track/playback/AudioFrame"),
        _ => return Err(jni::errors::Error::NullPtr("unknown proxy kind")),
    };
    let handler = env.new_object(
        HANDLER,
        jni_sig!("(ILjava/lang/String;)V"),
        &[JValue::Int(kind), JValue::Object(identifier)],
    )?;
    let interface = env.find_class(interface_name)?;
    let loader = env
        .call_method(
            &interface,
            jni_str!("getClassLoader"),
            jni_sig!("()Ljava/lang/ClassLoader;"),
            &[],
        )?
        .l()?;
    let interfaces = env.new_object_array(1, jni_str!("java/lang/Class"), JObject::null())?;
    interfaces.set_element(env, 0, &interface)?;
    env.call_static_method(
        jni_str!("java/lang/reflect/Proxy"),
        jni_str!("newProxyInstance"),
        jni_sig!("(Ljava/lang/ClassLoader;[Ljava/lang/Class;Ljava/lang/reflect/InvocationHandler;)Ljava/lang/Object;"),
        &[
            JValue::Object(&loader),
            JValue::Object(interfaces.as_ref()),
            JValue::Object(&handler),
        ],
    )?
    .l()
}

pub(crate) fn invoke<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
    proxy: &JObject<'local>,
    method: &JObject<'local>,
    args: &JObjectArray<'local, JObject<'local>>,
) -> jni::errors::Result<JObject<'local>> {
    let name = env
        .call_method(
            method,
            jni_str!("getName"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )?
        .l()?;
    let name = JString::cast_local(env, name)?.try_to_string(env)?;
    let kind = env
        .get_field(handler, jni_str!("kind"), jni_sig!("I"))?
        .i()?;

    match name.as_ref() {
        "toString" => {
            return Ok(env
                .new_string(format!("MantleGateProxy(kind={kind})"))?
                .into());
        }
        "hashCode" => {
            let hash = env
                .call_static_method(
                    jni_str!("java/lang/System"),
                    jni_str!("identityHashCode"),
                    jni_sig!("(Ljava/lang/Object;)I"),
                    &[JValue::Object(proxy)],
                )?
                .i()?;
            return box_int(env, hash);
        }
        "equals" => {
            let other = argument(args, env, 0)?;
            let same = env.is_same_object(proxy, &other)?;
            return box_bool(env, same);
        }
        _ => {}
    }

    match kind {
        2 => invoke_player(env, handler, proxy, args, name.as_ref()),
        3 => invoke_track(env, handler, args, name.as_ref()),
        4 => invoke_frame(env, handler, name.as_ref()),
        _ => Err(jni::errors::Error::NullPtr(
            "unknown invocation handler kind",
        )),
    }
}

fn invoke_player<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
    proxy: &JObject<'local>,
    args: &JObjectArray<'local, JObject<'local>>,
    name: &str,
) -> jni::errors::Result<JObject<'local>> {
    match name {
        "addListener" | "removeListener" => {
            let listener = argument(args, env, 0)?;
            let listeners = object_field(env, handler, "listeners", "Ljava/util/ArrayList;")?;
            let operation = if name == "addListener" {
                "add"
            } else {
                "remove"
            };
            let _ = env.call_method(
                &listeners,
                jni::strings::JNIString::from(operation),
                jni_sig!("(Ljava/lang/Object;)Z"),
                &[JValue::Object(&listener)],
            )?;
            Ok(JObject::null())
        }
        "playTrack" | "startTrack" => {
            let track = argument(args, env, 0)?;
            set_object_field(env, handler, "track", "Ljava/lang/Object;", &track)?;
            dispatch_track_start(env, handler, proxy, &track)?;
            if name == "startTrack" {
                box_bool(env, true)
            } else {
                Ok(JObject::null())
            }
        }
        "getPlayingTrack" => object_field(env, handler, "track", "Ljava/lang/Object;"),
        "setPaused" => {
            let paused = argument(args, env, 0)?;
            let paused = env
                .call_method(&paused, jni_str!("booleanValue"), jni_sig!("()Z"), &[])?
                .z()?;
            env.set_field(
                handler,
                jni_str!("paused"),
                jni_sig!("Z"),
                JValue::Bool(paused),
            )?;
            Ok(JObject::null())
        }
        "isPaused" => {
            let paused = env
                .get_field(handler, jni_str!("paused"), jni_sig!("Z"))?
                .z()?;
            box_bool(env, paused)
        }
        "setVolume" => {
            let volume = argument(args, env, 0)?;
            let volume = env
                .call_method(&volume, jni_str!("intValue"), jni_sig!("()I"), &[])?
                .i()?;
            env.set_field(
                handler,
                jni_str!("volume"),
                jni_sig!("I"),
                JValue::Int(volume),
            )?;
            Ok(JObject::null())
        }
        "getVolume" => {
            let volume = env
                .get_field(handler, jni_str!("volume"), jni_sig!("I"))?
                .i()?;
            box_int(env, volume)
        }
        "provide" => {
            let delivered = env
                .get_field(handler, jni_str!("delivered"), jni_sig!("Z"))?
                .z()?;
            if delivered {
                Ok(JObject::null())
            } else {
                env.set_field(
                    handler,
                    jni_str!("delivered"),
                    jni_sig!("Z"),
                    JValue::Bool(true),
                )?;
                create(env, 4, &JObject::null())
            }
        }
        "stopTrack" => {
            set_object_field(
                env,
                handler,
                "track",
                "Ljava/lang/Object;",
                &JObject::null(),
            )?;
            Ok(JObject::null())
        }
        "destroy" => {
            clean(env, handler)?;
            Ok(JObject::null())
        }
        "checkCleanup" | "setFilterFactory" | "setFrameBufferDuration" => Ok(JObject::null()),
        _ => unsupported(env, name),
    }
}

fn invoke_track<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
    args: &JObjectArray<'local, JObject<'local>>,
    name: &str,
) -> jni::errors::Result<JObject<'local>> {
    match name {
        "getIdentifier" => object_field(env, handler, "identifier", "Ljava/lang/String;"),
        "setUserData" => {
            let value = argument(args, env, 0)?;
            set_object_field(env, handler, "userData", "Ljava/lang/Object;", &value)?;
            Ok(JObject::null())
        }
        "getUserData" => {
            let value = object_field(env, handler, "userData", "Ljava/lang/Object;")?;
            if args.is_null() || args.len(env)? == 0 || value.is_null() {
                return Ok(value);
            }
            let class = argument(args, env, 0)?;
            if env
                .call_method(
                    &class,
                    jni_str!("isInstance"),
                    jni_sig!("(Ljava/lang/Object;)Z"),
                    &[JValue::Object(&value)],
                )?
                .z()?
            {
                Ok(value)
            } else {
                Ok(JObject::null())
            }
        }
        "isSeekable" => box_bool(env, true),
        "getPosition" => {
            let position = env
                .get_field(handler, jni_str!("position"), jni_sig!("J"))?
                .j()?;
            box_long(env, position)
        }
        "setPosition" => {
            let value = argument(args, env, 0)?;
            let position = env
                .call_method(&value, jni_str!("longValue"), jni_sig!("()J"), &[])?
                .j()?;
            env.set_field(
                handler,
                jni_str!("position"),
                jni_sig!("J"),
                JValue::Long(position),
            )?;
            fire_marker(env, handler, position)?;
            Ok(JObject::null())
        }
        "setMarker" | "addMarker" => {
            let marker = argument(args, env, 0)?;
            set_object_field(env, handler, "marker", "Ljava/lang/Object;", &marker)?;
            Ok(JObject::null())
        }
        "removeMarker" => {
            let requested = argument(args, env, 0)?;
            let marker = object_field(env, handler, "marker", "Ljava/lang/Object;")?;
            if env.is_same_object(&requested, &marker)? {
                set_object_field(
                    env,
                    handler,
                    "marker",
                    "Ljava/lang/Object;",
                    &JObject::null(),
                )?;
            }
            Ok(JObject::null())
        }
        "getDuration" => box_long(env, 1_000),
        "makeClone" => {
            let identifier = object_field(env, handler, "identifier", "Ljava/lang/String;")?;
            create(env, 3, &identifier)
        }
        "stop" => {
            clean(env, handler)?;
            Ok(JObject::null())
        }
        "getInfo" | "getState" | "getSourceManager" => Ok(JObject::null()),
        _ => unsupported(env, name),
    }
}

fn invoke_frame<'local>(
    env: &mut Env<'local>,
    _handler: &JObject<'local>,
    name: &str,
) -> jni::errors::Result<JObject<'local>> {
    match name {
        "getTimecode" => box_long(env, 0),
        "getVolume" => box_int(env, 100),
        "getDataLength" => box_int(env, 4),
        "getData" => Ok(env.byte_array_from_slice(&[1, 2, 3, 4])?.into()),
        "getFormat" => Ok(JObject::null()),
        "isTerminator" => box_bool(env, false),
        _ => unsupported(env, name),
    }
}

fn dispatch_track_start<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
    player: &JObject<'local>,
    track: &JObject<'local>,
) -> jni::errors::Result<()> {
    let event = env.new_object(
        jni_str!("com/sedmelluq/discord/lavaplayer/player/event/TrackStartEvent"),
        jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;)V"),
        &[JValue::Object(player), JValue::Object(track)],
    )?;
    let listeners = object_field(env, handler, "listeners", "Ljava/util/ArrayList;")?;
    let snapshot = env
        .call_method(
            &listeners,
            jni_str!("toArray"),
            jni_sig!("()[Ljava/lang/Object;"),
            &[],
        )?
        .l()?;
    let snapshot = JObjectArray::<JObject>::cast_local(env, snapshot)?;
    for index in 0..snapshot.len(env)? {
        let listener = snapshot.get_element(env, index)?;
        let _ = env.call_method(
            &listener,
            jni_str!("onEvent"),
            jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/player/event/AudioEvent;)V"),
            &[JValue::Object(&event)],
        )?;
    }
    Ok(())
}

fn fire_marker<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
    position: i64,
) -> jni::errors::Result<()> {
    let marker = object_field(env, handler, "marker", "Ljava/lang/Object;")?;
    if marker.is_null() {
        return Ok(());
    }
    let timecode = env
        .get_field(&marker, jni_str!("timecode"), jni_sig!("J"))?
        .j()?;
    if position < timecode {
        return Ok(());
    }
    let callback = env
        .get_field(
            &marker,
            jni_str!("handler"),
            jni_sig!("Lcom/sedmelluq/discord/lavaplayer/track/TrackMarkerHandler;"),
        )?
        .l()?;
    let state_class = env.find_class(jni_str!(
        "com/sedmelluq/discord/lavaplayer/track/TrackMarkerHandler$MarkerState"
    ))?;
    let mut state = env
        .get_static_field(
            &state_class,
            jni_str!("REACHED"),
            jni_sig!("Lcom/sedmelluq/discord/lavaplayer/track/TrackMarkerHandler$MarkerState;"),
        )?
        .l()?;
    if state.is_null() {
        let name = env.new_string("REACHED")?;
        state = env.new_object(
            &state_class,
            jni_sig!("(Ljava/lang/String;I)V"),
            &[JValue::Object(name.as_ref()), JValue::Int(0)],
        )?;
        env.set_static_field(
            &state_class,
            jni_str!("REACHED"),
            jni_sig!("Lcom/sedmelluq/discord/lavaplayer/track/TrackMarkerHandler$MarkerState;"),
            JValue::Object(&state),
        )?;
    }
    let _ = env.call_method(
        &callback,
        jni_str!("handle"),
        jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/track/TrackMarkerHandler$MarkerState;)V"),
        &[JValue::Object(&state)],
    )?;
    set_object_field(
        env,
        handler,
        "marker",
        "Ljava/lang/Object;",
        &JObject::null(),
    )
}

fn clean<'local>(env: &mut Env<'local>, handler: &JObject<'local>) -> jni::errors::Result<()> {
    let cleanable = object_field(
        env,
        handler,
        "cleanable",
        "Ljava/lang/ref/Cleaner$Cleanable;",
    )?;
    let _ = env.call_method(&cleanable, jni_str!("clean"), jni_sig!("()V"), &[])?;
    Ok(())
}

fn argument<'local>(
    args: &JObjectArray<'local, JObject<'local>>,
    env: &mut Env<'local>,
    index: usize,
) -> jni::errors::Result<JObject<'local>> {
    args.get_element(env, index)
}

fn object_field<'local>(
    env: &mut Env<'local>,
    object: &JObject<'local>,
    name: &str,
    descriptor: &str,
) -> jni::errors::Result<JObject<'local>> {
    match (name, descriptor) {
        ("identifier", "Ljava/lang/String;") => env
            .get_field(
                object,
                jni_str!("identifier"),
                jni_sig!("Ljava/lang/String;"),
            )?
            .l(),
        ("userData", "Ljava/lang/Object;") => env
            .get_field(object, jni_str!("userData"), jni_sig!("Ljava/lang/Object;"))?
            .l(),
        ("marker", "Ljava/lang/Object;") => env
            .get_field(object, jni_str!("marker"), jni_sig!("Ljava/lang/Object;"))?
            .l(),
        ("track", "Ljava/lang/Object;") => env
            .get_field(object, jni_str!("track"), jni_sig!("Ljava/lang/Object;"))?
            .l(),
        ("listeners", "Ljava/util/ArrayList;") => env
            .get_field(
                object,
                jni_str!("listeners"),
                jni_sig!("Ljava/util/ArrayList;"),
            )?
            .l(),
        ("cleanable", "Ljava/lang/ref/Cleaner$Cleanable;") => env
            .get_field(
                object,
                jni_str!("cleanable"),
                jni_sig!("Ljava/lang/ref/Cleaner$Cleanable;"),
            )?
            .l(),
        _ => Err(jni::errors::Error::NullPtr(
            "unknown generated object field",
        )),
    }
}

fn set_object_field<'local>(
    env: &mut Env<'local>,
    object: &JObject<'local>,
    name: &str,
    descriptor: &str,
    value: &JObject<'local>,
) -> jni::errors::Result<()> {
    match (name, descriptor) {
        ("userData", "Ljava/lang/Object;") => env.set_field(
            object,
            jni_str!("userData"),
            jni_sig!("Ljava/lang/Object;"),
            JValue::Object(value),
        ),
        ("marker", "Ljava/lang/Object;") => env.set_field(
            object,
            jni_str!("marker"),
            jni_sig!("Ljava/lang/Object;"),
            JValue::Object(value),
        ),
        ("track", "Ljava/lang/Object;") => env.set_field(
            object,
            jni_str!("track"),
            jni_sig!("Ljava/lang/Object;"),
            JValue::Object(value),
        ),
        _ => Err(jni::errors::Error::NullPtr(
            "unknown generated object field",
        )),
    }
}

fn box_bool<'local>(env: &mut Env<'local>, value: bool) -> jni::errors::Result<JObject<'local>> {
    env.call_static_method(
        jni_str!("java/lang/Boolean"),
        jni_str!("valueOf"),
        jni_sig!("(Z)Ljava/lang/Boolean;"),
        &[JValue::Bool(value)],
    )?
    .l()
}

fn box_int<'local>(env: &mut Env<'local>, value: i32) -> jni::errors::Result<JObject<'local>> {
    env.call_static_method(
        jni_str!("java/lang/Integer"),
        jni_str!("valueOf"),
        jni_sig!("(I)Ljava/lang/Integer;"),
        &[JValue::Int(value)],
    )?
    .l()
}

fn box_long<'local>(env: &mut Env<'local>, value: i64) -> jni::errors::Result<JObject<'local>> {
    env.call_static_method(
        jni_str!("java/lang/Long"),
        jni_str!("valueOf"),
        jni_sig!("(J)Ljava/lang/Long;"),
        &[JValue::Long(value)],
    )?
    .l()
}

fn unsupported<'local>(env: &mut Env<'local>, name: &str) -> jni::errors::Result<JObject<'local>> {
    env.throw_new(
        jni_str!("java/lang/UnsupportedOperationException"),
        jni::strings::JNIString::from(format!("Gate A proxy does not implement {name}")),
    )?;
    Err(jni::errors::Error::JavaException)
}
