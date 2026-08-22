use jni::objects::{JObject, JObjectArray, JString, JValue};
use jni::{Env, jni_sig, jni_str};
use mantle_core::{
    EndReason, Event, MarkerId, MarkerSignal, MarkerState, PlayerId, TrackId, TrackState,
    UserDataToken,
};
use std::time::Duration;

use crate::registry::{CoreObject, HandleKind};

const HANDLER: &jni::strings::JNIStr = jni_str!("dev/mantle/internal/NativeInvocationHandler");

pub(crate) fn create<'local>(
    env: &mut Env<'local>,
    kind: i32,
    identifier: &JObject<'local>,
) -> jni::errors::Result<JObject<'local>> {
    let interface_name = match kind {
        2 => jni_str!("com/sedmelluq/discord/lavaplayer/player/AudioPlayer"),
        3 => jni_str!("com/sedmelluq/discord/lavaplayer/track/InternalAudioTrack"),
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
        "addListener" | "removeListener" => player_listener(env, handler, args, name),
        "playTrack" | "startTrack" => player_start(env, handler, proxy, args, name),
        "getPlayingTrack" => object_field(env, handler, "track", "Ljava/lang/Object;"),
        "setPaused" => player_set_paused(env, handler, proxy, args),
        "isPaused" => {
            let player = player_id(env, handler)?;
            box_bool(
                env,
                crate::with_engine(|engine| engine.player_paused(player))
                    .map_err(|_| jni::errors::Error::NullPtr("core pause query failed"))?,
            )
        }
        "setVolume" => player_set_volume(env, handler, args),
        "getVolume" => {
            let player = player_id(env, handler)?;
            box_int(
                env,
                i32::from(
                    crate::with_engine(|engine| engine.player_volume(player))
                        .map_err(|_| jni::errors::Error::NullPtr("core volume query failed"))?,
                ),
            )
        }
        "provide" => player_provide(env, handler, proxy, args),
        "stopTrack" => player_stop(env, handler, proxy),
        "destroy" => {
            clean(env, handler)?;
            Ok(JObject::null())
        }
        "checkCleanup" | "setFilterFactory" | "setFrameBufferDuration" => Ok(JObject::null()),
        _ => unsupported(env, name),
    }
}

fn player_listener<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
    args: &JObjectArray<'local, JObject<'local>>,
    name: &str,
) -> jni::errors::Result<JObject<'local>> {
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

fn player_start<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
    proxy: &JObject<'local>,
    args: &JObjectArray<'local, JObject<'local>>,
    name: &str,
) -> jni::errors::Result<JObject<'local>> {
    let track = argument(args, env, 0)?;
    let player_id = player_id(env, handler)?;
    let track_id = track_id_from_proxy(env, &track)?;
    let no_interrupt = if name == "startTrack" {
        let value = argument(args, env, 1)?;
        env.call_method(&value, jni_str!("booleanValue"), jni_sig!("()Z"), &[])?
            .z()?
    } else {
        false
    };
    let previous = object_field(env, handler, "track", "Ljava/lang/Object;")?;
    let (started, transition) =
        crate::with_engine(|engine| engine.start_track(player_id, track_id, no_interrupt))
            .map_err(|_| jni::errors::Error::NullPtr("core start-track transition failed"))?;
    if !started {
        return box_bool(env, false);
    }
    set_object_field(env, handler, "track", "Ljava/lang/Object;", &track)?;
    dispatch_transition(env, handler, proxy, &previous, &track, &transition.events)?;
    if name == "startTrack" {
        box_bool(env, true)
    } else {
        Ok(JObject::null())
    }
}

fn player_set_paused<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
    proxy: &JObject<'local>,
    args: &JObjectArray<'local, JObject<'local>>,
) -> jni::errors::Result<JObject<'local>> {
    let paused = argument(args, env, 0)?;
    let paused = env
        .call_method(&paused, jni_str!("booleanValue"), jni_sig!("()Z"), &[])?
        .z()?;
    let player = player_id(env, handler)?;
    let transition = crate::with_engine(|engine| engine.set_paused(player, paused))
        .map_err(|_| jni::errors::Error::NullPtr("core pause transition failed"))?;
    dispatch_transition(
        env,
        handler,
        proxy,
        &JObject::null(),
        &JObject::null(),
        &transition.events,
    )?;
    Ok(JObject::null())
}

fn player_set_volume<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
    args: &JObjectArray<'local, JObject<'local>>,
) -> jni::errors::Result<JObject<'local>> {
    let volume = argument(args, env, 0)?;
    let volume = env
        .call_method(&volume, jni_str!("intValue"), jni_sig!("()I"), &[])?
        .i()?;
    let player = player_id(env, handler)?;
    crate::with_engine(|engine| engine.set_volume(player, volume))
        .map_err(|_| jni::errors::Error::NullPtr("core volume update failed"))?;
    Ok(JObject::null())
}

fn player_provide<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
    proxy: &JObject<'local>,
    args: &JObjectArray<'local, JObject<'local>>,
) -> jni::errors::Result<JObject<'local>> {
    let timeout = if args.is_null() || args.len(env)? == 0 {
        Duration::ZERO
    } else {
        let value = argument(args, env, 0)?;
        let millis = env
            .call_method(&value, jni_str!("longValue"), jni_sig!("()J"), &[])?
            .j()?;
        Duration::from_millis(u64::try_from(millis.max(0)).unwrap_or(u64::MAX))
    };
    let player = player_id(env, handler)?;
    let (frame, transition) = crate::with_engine(|engine| engine.provide(player, timeout))
        .map_err(|_| jni::errors::Error::NullPtr("core frame provision failed"))?;
    let active = object_field(env, handler, "track", "Ljava/lang/Object;")?;
    dispatch_transition(env, handler, proxy, &active, &active, &transition.events)?;
    frame.map_or_else(|| Ok(JObject::null()), |_| create(env, 4, &JObject::null()))
}

fn player_stop<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
    proxy: &JObject<'local>,
) -> jni::errors::Result<JObject<'local>> {
    let previous = object_field(env, handler, "track", "Ljava/lang/Object;")?;
    let player = player_id(env, handler)?;
    let transition = crate::with_engine(|engine| engine.stop_track(player, EndReason::Stopped))
        .map_err(|_| jni::errors::Error::NullPtr("core stop transition failed"))?;
    set_object_field(
        env,
        handler,
        "track",
        "Ljava/lang/Object;",
        &JObject::null(),
    )?;
    dispatch_transition(
        env,
        handler,
        proxy,
        &previous,
        &JObject::null(),
        &transition.events,
    )?;
    Ok(JObject::null())
}

fn invoke_track<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
    args: &JObjectArray<'local, JObject<'local>>,
    name: &str,
) -> jni::errors::Result<JObject<'local>> {
    match name {
        "getIdentifier" => object_field(env, handler, "identifier", "Ljava/lang/String;"),
        "setUserData" => track_set_user_data(env, handler, args),
        "getUserData" => track_get_user_data(env, handler, args),
        "isSeekable" => box_bool(env, true),
        "getPosition" => {
            let track = track_id(env, handler)?;
            box_long(
                env,
                duration_millis_i64(
                    crate::with_engine(|engine| engine.track(track).map(|track| track.position))
                        .map_err(|_| jni::errors::Error::NullPtr("core position query failed"))?,
                ),
            )
        }
        "setPosition" => track_set_position(env, handler, args),
        "setMarker" | "addMarker" => track_set_marker(env, handler, args, name),
        "removeMarker" => track_remove_marker(env, handler, args),
        "getDuration" => {
            let track = track_id(env, handler)?;
            box_long(
                env,
                duration_millis_i64(
                    crate::with_engine(|engine| {
                        engine.track(track).map(|track| track.info.duration)
                    })
                    .map_err(|_| jni::errors::Error::NullPtr("core duration query failed"))?,
                ),
            )
        }
        "getInfo" => track_info(env, handler),
        "getState" => track_state(env, handler),
        "makeClone" => {
            let identifier = object_field(env, handler, "identifier", "Ljava/lang/String;")?;
            create(env, 3, &identifier)
        }
        "stop" => {
            clean(env, handler)?;
            Ok(JObject::null())
        }
        "getSourceManager"
        | "assignExecutor"
        | "process"
        | "getActiveExecutor"
        | "createLocalExecutor" => Ok(JObject::null()),
        "provide" => track_provide(env, handler),
        _ => unsupported(env, name),
    }
}

fn track_provide<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
) -> jni::errors::Result<JObject<'local>> {
    let track = track_id(env, handler)?;
    let frame = crate::with_engine(|engine| engine.provide_track(track))
        .map_err(|_| jni::errors::Error::NullPtr("core track frame provision failed"))?;
    frame.map_or_else(|| Ok(JObject::null()), |_| create(env, 4, &JObject::null()))
}

fn track_set_user_data<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
    args: &JObjectArray<'local, JObject<'local>>,
) -> jni::errors::Result<JObject<'local>> {
    let value = argument(args, env, 0)?;
    let token = (!value.is_null()).then(|| UserDataToken::from_opaque(1));
    let track = track_id(env, handler)?;
    crate::with_engine(|engine| engine.set_user_data(track, token))
        .map_err(|_| jni::errors::Error::NullPtr("core user-data update failed"))?;
    set_object_field(env, handler, "userData", "Ljava/lang/Object;", &value)?;
    Ok(JObject::null())
}

fn track_get_user_data<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
    args: &JObjectArray<'local, JObject<'local>>,
) -> jni::errors::Result<JObject<'local>> {
    let value = object_field(env, handler, "userData", "Ljava/lang/Object;")?;
    if args.is_null() || args.len(env)? == 0 || value.is_null() {
        return Ok(value);
    }
    let class = argument(args, env, 0)?;
    let matches = env
        .call_method(
            &class,
            jni_str!("isInstance"),
            jni_sig!("(Ljava/lang/Object;)Z"),
            &[JValue::Object(&value)],
        )?
        .z()?;
    Ok(if matches { value } else { JObject::null() })
}

fn track_set_position<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
    args: &JObjectArray<'local, JObject<'local>>,
) -> jni::errors::Result<JObject<'local>> {
    let value = argument(args, env, 0)?;
    let position = env
        .call_method(&value, jni_str!("longValue"), jni_sig!("()J"), &[])?
        .j()?;
    let track = track_id(env, handler)?;
    let signals = crate::with_engine(|engine| {
        engine.seek(
            track,
            Duration::from_millis(u64::try_from(position.max(0)).unwrap_or(u64::MAX)),
        )
    })
    .map_err(|_| jni::errors::Error::NullPtr("core seek transition failed"))?;
    fire_marker_signals(env, handler, &signals)?;
    Ok(JObject::null())
}

fn track_set_marker<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
    args: &JObjectArray<'local, JObject<'local>>,
    name: &str,
) -> jni::errors::Result<JObject<'local>> {
    let marker = argument(args, env, 0)?;
    let timecode = env
        .get_field(&marker, jni_str!("timecode"), jni_sig!("J"))?
        .j()?;
    let id = MarkerId::from_opaque(1);
    let track = track_id(env, handler)?;
    let timecode = Duration::from_millis(u64::try_from(timecode.max(0)).unwrap_or(u64::MAX));
    let signals = if name == "setMarker" {
        crate::with_engine(|engine| engine.set_marker(track, Some((id, timecode))))
    } else {
        crate::with_engine(|engine| engine.add_marker(track, id, timecode))
    }
    .map_err(|_| jni::errors::Error::NullPtr("core marker update failed"))?;
    fire_marker_signals(env, handler, &signals)?;
    set_object_field(env, handler, "marker", "Ljava/lang/Object;", &marker)?;
    Ok(JObject::null())
}

fn track_remove_marker<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
    args: &JObjectArray<'local, JObject<'local>>,
) -> jni::errors::Result<JObject<'local>> {
    let requested = argument(args, env, 0)?;
    let marker = object_field(env, handler, "marker", "Ljava/lang/Object;")?;
    if env.is_same_object(&requested, &marker)? {
        let track = track_id(env, handler)?;
        let signals =
            crate::with_engine(|engine| engine.remove_marker(track, MarkerId::from_opaque(1)))
                .map_err(|_| jni::errors::Error::NullPtr("core marker removal failed"))?;
        fire_marker_signals(env, handler, &signals)?;
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

fn dispatch_transition<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
    player: &JObject<'local>,
    previous_track: &JObject<'local>,
    new_track: &JObject<'local>,
    events: &[Event],
) -> jni::errors::Result<()> {
    for event in events {
        let event = match event {
            Event::TrackStart { .. } => env.new_object(
                jni_str!("com/sedmelluq/discord/lavaplayer/player/event/TrackStartEvent"),
                jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;)V"),
                &[JValue::Object(player), JValue::Object(new_track)],
            )?,
            Event::TrackEnd { reason, .. } => {
                let reason = end_reason(env, *reason)?;
                env.new_object(
                    jni_str!("com/sedmelluq/discord/lavaplayer/player/event/TrackEndEvent"),
                    jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackEndReason;)V"),
                    &[
                        JValue::Object(player),
                        JValue::Object(previous_track),
                        JValue::Object(&reason),
                    ],
                )?
            }
            Event::PlayerPause { .. } => env.new_object(
                jni_str!("com/sedmelluq/discord/lavaplayer/player/event/PlayerPauseEvent"),
                jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;)V"),
                &[JValue::Object(player)],
            )?,
            Event::PlayerResume { .. } => env.new_object(
                jni_str!("com/sedmelluq/discord/lavaplayer/player/event/PlayerResumeEvent"),
                jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;)V"),
                &[JValue::Object(player)],
            )?,
            Event::TrackStuck { .. } => continue,
        };
        dispatch_event(env, handler, &event)?;
    }
    Ok(())
}

fn dispatch_event<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
    event: &JObject<'local>,
) -> jni::errors::Result<()> {
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
            &[JValue::Object(event)],
        )?;
    }
    Ok(())
}

fn fire_marker_signals<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
    signals: &[MarkerSignal],
) -> jni::errors::Result<()> {
    if signals.is_empty() {
        return Ok(());
    }
    let marker = object_field(env, handler, "marker", "Ljava/lang/Object;")?;
    if marker.is_null() {
        return Ok(());
    }
    let callback = env
        .get_field(
            &marker,
            jni_str!("handler"),
            jni_sig!("Lcom/sedmelluq/discord/lavaplayer/track/TrackMarkerHandler;"),
        )?
        .l()?;
    for signal in signals {
        let (name, ordinal) = marker_state_name(signal.state);
        let state = enum_constant(
            env,
            "com/sedmelluq/discord/lavaplayer/track/TrackMarkerHandler$MarkerState",
            name,
            ordinal,
            "Lcom/sedmelluq/discord/lavaplayer/track/TrackMarkerHandler$MarkerState;",
        )?;
        let _ = env.call_method(
            &callback,
            jni_str!("handle"),
            jni_sig!("(Lcom/sedmelluq/discord/lavaplayer/track/TrackMarkerHandler$MarkerState;)V"),
            &[JValue::Object(&state)],
        )?;
    }
    set_object_field(
        env,
        handler,
        "marker",
        "Ljava/lang/Object;",
        &JObject::null(),
    )
}

fn marker_state_name(state: MarkerState) -> (&'static str, i32) {
    match state {
        MarkerState::Reached => ("REACHED", 0),
        MarkerState::Bypassed => ("BYPASSED", 1),
        MarkerState::Removed => ("REMOVED", 2),
        MarkerState::Overwritten => ("OVERWRITTEN", 3),
        MarkerState::Late => ("LATE", 4),
        MarkerState::Stopped => ("STOPPED", 5),
        MarkerState::Ended => ("ENDED", 6),
    }
}

fn end_reason<'local>(
    env: &mut Env<'local>,
    reason: EndReason,
) -> jni::errors::Result<JObject<'local>> {
    let (name, ordinal, may_start_next) = match reason {
        EndReason::Finished => ("FINISHED", 0, true),
        EndReason::LoadFailed => ("LOAD_FAILED", 1, true),
        EndReason::Stopped => ("STOPPED", 2, false),
        EndReason::Replaced => ("REPLACED", 3, false),
        EndReason::Cleanup => ("CLEANUP", 4, false),
    };
    let class = env.find_class(jni_str!(
        "com/sedmelluq/discord/lavaplayer/track/AudioTrackEndReason"
    ))?;
    let mut value = env
        .get_static_field(
            &class,
            jni::strings::JNIString::from(name),
            jni_sig!("Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackEndReason;"),
        )?
        .l()?;
    if value.is_null() {
        let enum_name = env.new_string(name)?;
        value = env.new_object(
            &class,
            jni_sig!("(Ljava/lang/String;IZ)V"),
            &[
                JValue::Object(enum_name.as_ref()),
                JValue::Int(ordinal),
                JValue::Bool(may_start_next),
            ],
        )?;
        env.set_static_field(
            &class,
            jni::strings::JNIString::from(name),
            jni_sig!("Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackEndReason;"),
            JValue::Object(&value),
        )?;
    }
    Ok(value)
}

fn enum_constant<'local>(
    env: &mut Env<'local>,
    class_name: &str,
    field_name: &str,
    ordinal: i32,
    descriptor: &str,
) -> jni::errors::Result<JObject<'local>> {
    let class = env.find_class(jni::strings::JNIString::from(class_name))?;
    let descriptor = jni::signature::RuntimeFieldSignature::from_str(descriptor)?;
    let descriptor = descriptor.field_signature();
    let mut value = env
        .get_static_field(
            &class,
            jni::strings::JNIString::from(field_name),
            descriptor.clone(),
        )?
        .l()?;
    if value.is_null() {
        let enum_name = env.new_string(field_name)?;
        value = env.new_object(
            &class,
            jni_sig!("(Ljava/lang/String;I)V"),
            &[JValue::Object(enum_name.as_ref()), JValue::Int(ordinal)],
        )?;
        env.set_static_field(
            &class,
            jni::strings::JNIString::from(field_name),
            descriptor,
            JValue::Object(&value),
        )?;
    }
    Ok(value)
}

fn track_info<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
) -> jni::errors::Result<JObject<'local>> {
    let id = track_id(env, handler)?;
    let info = crate::with_engine(|engine| engine.track(id).map(|track| track.info.clone()))
        .map_err(|_| jni::errors::Error::NullPtr("core track metadata query failed"))?;
    let title = env.new_string(info.title)?;
    let author = env.new_string(info.author)?;
    let identifier = env.new_string(info.identifier)?;
    let uri = optional_string(env, info.uri.as_deref())?;
    let artwork = optional_string(env, info.artwork_url.as_deref())?;
    let isrc = optional_string(env, info.isrc.as_deref())?;
    env.new_object(
        jni_str!("com/sedmelluq/discord/lavaplayer/track/AudioTrackInfo"),
        jni_sig!("(Ljava/lang/String;Ljava/lang/String;JLjava/lang/String;ZLjava/lang/String;Ljava/lang/String;Ljava/lang/String;)V"),
        &[
            JValue::Object(title.as_ref()),
            JValue::Object(author.as_ref()),
            JValue::Long(duration_millis_i64(info.duration)),
            JValue::Object(identifier.as_ref()),
            JValue::Bool(info.is_stream),
            JValue::Object(&uri),
            JValue::Object(&artwork),
            JValue::Object(&isrc),
        ],
    )
}

fn optional_string<'local>(
    env: &mut Env<'local>,
    value: Option<&str>,
) -> jni::errors::Result<JObject<'local>> {
    value.map_or_else(
        || Ok(JObject::null()),
        |value| env.new_string(value).map(Into::into),
    )
}

fn track_state<'local>(
    env: &mut Env<'local>,
    handler: &JObject<'local>,
) -> jni::errors::Result<JObject<'local>> {
    let id = track_id(env, handler)?;
    let state = crate::with_engine(|engine| engine.track(id).map(|track| track.state))
        .map_err(|_| jni::errors::Error::NullPtr("core track state query failed"))?;
    let (name, ordinal) = match state {
        TrackState::Inactive => ("INACTIVE", 0),
        TrackState::Loading => ("LOADING", 1),
        TrackState::Playing => ("PLAYING", 2),
        TrackState::Seeking => ("SEEKING", 3),
        TrackState::Stopping => ("STOPPING", 4),
        TrackState::Finished => ("FINISHED", 5),
    };
    enum_constant(
        env,
        "com/sedmelluq/discord/lavaplayer/track/AudioTrackState",
        name,
        ordinal,
        "Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackState;",
    )
}

fn player_id(env: &mut Env<'_>, handler: &JObject<'_>) -> jni::errors::Result<PlayerId> {
    match core_object(env, handler, HandleKind::Player)? {
        CoreObject::Player(id) => Ok(id),
        _ => Err(jni::errors::Error::NullPtr("proxy is not a core player")),
    }
}

fn track_id(env: &mut Env<'_>, handler: &JObject<'_>) -> jni::errors::Result<TrackId> {
    match core_object(env, handler, HandleKind::Track)? {
        CoreObject::Track(id) => Ok(id),
        _ => Err(jni::errors::Error::NullPtr("proxy is not a core track")),
    }
}

fn core_object(
    env: &mut Env<'_>,
    handler: &JObject<'_>,
    kind: HandleKind,
) -> jni::errors::Result<CoreObject> {
    let raw = env
        .get_field(handler, jni_str!("handle"), jni_sig!("J"))?
        .j()?;
    crate::core_for_handle(raw, kind)
}

pub(crate) fn track_id_from_proxy(
    env: &mut Env<'_>,
    track: &JObject<'_>,
) -> jni::errors::Result<TrackId> {
    let handler = env
        .call_static_method(
            jni_str!("java/lang/reflect/Proxy"),
            jni_str!("getInvocationHandler"),
            jni_sig!("(Ljava/lang/Object;)Ljava/lang/reflect/InvocationHandler;"),
            &[JValue::Object(track)],
        )?
        .l()?;
    track_id(env, &handler)
}

fn duration_millis_i64(duration: Duration) -> i64 {
    i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
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
