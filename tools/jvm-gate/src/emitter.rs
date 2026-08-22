use std::error::Error;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

use ristretto_classfile::attributes::{
    ArrayType, Attribute, ExceptionTableEntry, Instruction, StackFrame, VerificationType,
};
use ristretto_classfile::{
    ClassAccessFlags, ClassFile, ConstantPool, Field, FieldAccessFlags, FieldType, JAVA_5, Method,
    MethodAccessFlags,
};
use serde::Serialize;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const NATIVE_CLASS: &str = "dev/mantle/internal/MantleNative";
const CLEANER_CLASS: &str = "dev/mantle/internal/NativeCleaner";
const STATE_CLASS: &str = "dev/mantle/internal/NativeState";
const PROBE_CLASS: &str = "dev/mantle/internal/NativeHandleProbe";
const HANDLER_CLASS: &str = "dev/mantle/internal/NativeInvocationHandler";
const LOAD_FUTURE_CLASS: &str = "dev/mantle/internal/NativeLoadFuture";
const LOAD_CALLBACK_CLASS: &str = "dev/mantle/internal/NativeLoadCallback";
const LOADER_CLASS: &str = "dev/mantle/internal/NativeLoader";
const FORMAT_CLASS: &str = "dev/mantle/internal/NativeAudioDataFormat";
const FRAME_BUFFER_FACTORY_CLASS: &str = "dev/mantle/internal/NativeAudioFrameBufferFactory";
const EVENT_DISPATCHER_CLASS: &str = "dev/mantle/internal/NativeEventDispatcher";
const PLAYER_LIFECYCLE_HELPER_CLASS: &str = "dev/mantle/internal/NativeAudioPlayerLifecycle";
const MANAGER_CLASS: &str = "com/sedmelluq/discord/lavaplayer/player/DefaultAudioPlayerManager";
const PLAYER_LIFECYCLE_MANAGER_CLASS: &str =
    "com/sedmelluq/discord/lavaplayer/player/AudioPlayerLifecycleManager";
const FUNCTIONAL_RESULT_HANDLER_CLASS: &str =
    "com/sedmelluq/discord/lavaplayer/player/FunctionalResultHandler";
const AUDIO_REFERENCE_CLASS: &str = "com/sedmelluq/discord/lavaplayer/track/AudioReference";
const DECODED_TRACK_HOLDER_CLASS: &str =
    "com/sedmelluq/discord/lavaplayer/track/DecodedTrackHolder";
const BASIC_PLAYLIST_CLASS: &str = "com/sedmelluq/discord/lavaplayer/track/BasicAudioPlaylist";
const CONFIGURATION_CLASS: &str = "com/sedmelluq/discord/lavaplayer/player/AudioConfiguration";
const AUDIO_PLAYER_OPTIONS_CLASS: &str =
    "com/sedmelluq/discord/lavaplayer/player/AudioPlayerOptions";
const RESAMPLING_CLASS: &str =
    "com/sedmelluq/discord/lavaplayer/player/AudioConfiguration$ResamplingQuality";
const AUDIO_FRAME_BUFFER_CLASS: &str =
    "com/sedmelluq/discord/lavaplayer/track/playback/AudioFrameBuffer";
const AUDIO_FRAME_CONSUMER_CLASS: &str =
    "com/sedmelluq/discord/lavaplayer/track/playback/AudioFrameConsumer";
const AUDIO_FRAME_REBUILDER_CLASS: &str =
    "com/sedmelluq/discord/lavaplayer/track/playback/AudioFrameRebuilder";
const AUDIO_FRAME_PROVIDER_TOOLS_CLASS: &str =
    "com/sedmelluq/discord/lavaplayer/track/playback/AudioFrameProviderTools";
const AUDIO_PROCESSING_CONTEXT_CLASS: &str =
    "com/sedmelluq/discord/lavaplayer/track/playback/AudioProcessingContext";
const TERMINATOR_FRAME_CLASS: &str =
    "com/sedmelluq/discord/lavaplayer/track/playback/TerminatorAudioFrame";
const AUDIO_FRAME_BUFFER_FACTORY_CLASS: &str =
    "com/sedmelluq/discord/lavaplayer/track/playback/AudioFrameBufferFactory";
const MARKER_STATE_CLASS: &str =
    "com/sedmelluq/discord/lavaplayer/track/TrackMarkerHandler$MarkerState";
const TRACK_STATE_CLASS: &str = "com/sedmelluq/discord/lavaplayer/track/AudioTrackState";
const TRACK_END_REASON_CLASS: &str = "com/sedmelluq/discord/lavaplayer/track/AudioTrackEndReason";
const ABSTRACT_MUTABLE_FRAME_CLASS: &str =
    "com/sedmelluq/discord/lavaplayer/track/playback/AbstractMutableAudioFrame";
const IMMUTABLE_FRAME_CLASS: &str =
    "com/sedmelluq/discord/lavaplayer/track/playback/ImmutableAudioFrame";
const MUTABLE_FRAME_CLASS: &str =
    "com/sedmelluq/discord/lavaplayer/track/playback/MutableAudioFrame";
const REFERENCE_MUTABLE_FRAME_CLASS: &str =
    "com/sedmelluq/discord/lavaplayer/track/playback/ReferenceMutableAudioFrame";
const EVENT_ADAPTER_CLASS: &str = "com/sedmelluq/discord/lavaplayer/player/event/AudioEventAdapter";
const TRACK_EXCEPTION_EVENT_CLASS: &str =
    "com/sedmelluq/discord/lavaplayer/player/event/TrackExceptionEvent";
const TRACK_STUCK_EVENT_CLASS: &str =
    "com/sedmelluq/discord/lavaplayer/player/event/TrackStuckEvent";

const REFERENCE_CLASSES: &[&str] = &[
    "com/sedmelluq/discord/lavaplayer/player/AudioLoadResultHandler",
    PLAYER_LIFECYCLE_MANAGER_CLASS,
    FUNCTIONAL_RESULT_HANDLER_CLASS,
    CONFIGURATION_CLASS,
    RESAMPLING_CLASS,
    AUDIO_PLAYER_OPTIONS_CLASS,
    "com/sedmelluq/discord/lavaplayer/player/AudioPlayer",
    "com/sedmelluq/discord/lavaplayer/player/AudioPlayerManager",
    MANAGER_CLASS,
    "com/sedmelluq/discord/lavaplayer/player/event/AudioEvent",
    EVENT_ADAPTER_CLASS,
    "com/sedmelluq/discord/lavaplayer/player/event/AudioEventListener",
    "com/sedmelluq/discord/lavaplayer/player/event/PlayerPauseEvent",
    "com/sedmelluq/discord/lavaplayer/player/event/PlayerResumeEvent",
    "com/sedmelluq/discord/lavaplayer/player/event/TrackEndEvent",
    TRACK_EXCEPTION_EVENT_CLASS,
    "com/sedmelluq/discord/lavaplayer/player/event/TrackStartEvent",
    TRACK_STUCK_EVENT_CLASS,
    "com/sedmelluq/discord/lavaplayer/player/hook/AudioOutputHook",
    "com/sedmelluq/discord/lavaplayer/player/hook/AudioOutputHookFactory",
    "com/sedmelluq/discord/lavaplayer/filter/PcmFilterFactory",
    "com/sedmelluq/discord/lavaplayer/format/AudioDataFormat",
    "com/sedmelluq/discord/lavaplayer/source/AudioSourceManager",
    "com/sedmelluq/discord/lavaplayer/tools/FriendlyException",
    "com/sedmelluq/discord/lavaplayer/track/AudioItem",
    AUDIO_REFERENCE_CLASS,
    "com/sedmelluq/discord/lavaplayer/track/AudioPlaylist",
    BASIC_PLAYLIST_CLASS,
    "com/sedmelluq/discord/lavaplayer/track/AudioTrack",
    "com/sedmelluq/discord/lavaplayer/track/AudioTrackEndReason",
    "com/sedmelluq/discord/lavaplayer/track/AudioTrackInfo",
    "com/sedmelluq/discord/lavaplayer/track/AudioTrackState",
    DECODED_TRACK_HOLDER_CLASS,
    "com/sedmelluq/discord/lavaplayer/track/info/AudioTrackInfoProvider",
    "com/sedmelluq/discord/lavaplayer/track/TrackMarker",
    "com/sedmelluq/discord/lavaplayer/track/TrackMarkerHandler",
    "com/sedmelluq/discord/lavaplayer/track/TrackMarkerHandler$MarkerState",
    "com/sedmelluq/discord/lavaplayer/track/TrackStateListener",
    "com/sedmelluq/discord/lavaplayer/track/playback/AudioFrame",
    "com/sedmelluq/discord/lavaplayer/track/playback/AudioFrameProvider",
    AUDIO_FRAME_PROVIDER_TOOLS_CLASS,
    AUDIO_PROCESSING_CONTEXT_CLASS,
    AUDIO_FRAME_CONSUMER_CLASS,
    AUDIO_FRAME_BUFFER_CLASS,
    AUDIO_FRAME_BUFFER_FACTORY_CLASS,
    AUDIO_FRAME_REBUILDER_CLASS,
    TERMINATOR_FRAME_CLASS,
    "com/sedmelluq/discord/lavaplayer/track/playback/AbstractMutableAudioFrame",
    "com/sedmelluq/discord/lavaplayer/track/playback/ImmutableAudioFrame",
    "com/sedmelluq/discord/lavaplayer/track/playback/MutableAudioFrame",
    REFERENCE_MUTABLE_FRAME_CLASS,
];

#[derive(Serialize)]
struct EmissionManifest {
    schema_version: u8,
    expected_native_abi: u8,
    reference_shells: Vec<ClassManifest>,
    internal_classes: Vec<String>,
}

#[derive(Serialize)]
struct ClassManifest {
    binary_name: String,
    classfile_major: u16,
    exported_fields: usize,
    exported_methods: usize,
}

pub fn emit(
    reference_jar: &Path,
    output: &Path,
    expected_abi: u8,
    manifest_output: Option<&Path>,
) -> Result<()> {
    let mut source = ZipArchive::new(File::open(reference_jar)?)?;
    let mut classes = Vec::new();
    for binary_name in REFERENCE_CLASSES {
        let mut entry = source.by_name(&format!("{binary_name}.class"))?;
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        let class = ClassFile::from_bytes(&bytes)?;
        classes.push(transform_reference_class(class)?);
    }
    classes.extend(internal_classes(expected_abi)?);

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut jar = ZipWriter::new(File::create(output)?);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    jar.start_file("META-INF/MANIFEST.MF", options)?;
    jar.write_all(b"Manifest-Version: 1.0\r\nCreated-By: Mantle Rust Gate A emitter\r\n\r\n")?;
    classes.sort_by(|left, right| {
        left.class_name()
            .expect("emitted class has a name")
            .cmp(right.class_name().expect("emitted class has a name"))
    });
    for class in &classes {
        class.verify().map_err(|error| {
            format!(
                "emitted class {} failed internal verification: {error}",
                class
                    .class_name()
                    .map_or_else(|_| "<unknown>".to_owned(), ToString::to_string)
            )
        })?;
        let name = format!("{}.class", class.class_name()?);
        let mut bytes = Vec::new();
        class.to_bytes(&mut bytes)?;
        jar.start_file(name, options)?;
        jar.write_all(&bytes)?;
    }
    jar.finish()?;
    if let Some(manifest_output) = manifest_output {
        if let Some(parent) = manifest_output.parent() {
            fs::create_dir_all(parent)?;
        }
        let reference_shells = classes
            .iter()
            .filter_map(|class| {
                let name = class.class_name().ok()?.to_string();
                REFERENCE_CLASSES
                    .contains(&name.as_str())
                    .then(|| ClassManifest {
                        binary_name: name,
                        classfile_major: class.version.major(),
                        exported_fields: class
                            .fields
                            .iter()
                            .filter(|field| {
                                field.access_flags.intersects(
                                    FieldAccessFlags::PUBLIC | FieldAccessFlags::PROTECTED,
                                )
                            })
                            .count(),
                        exported_methods: class
                            .methods
                            .iter()
                            .filter(|method| {
                                method.access_flags.intersects(
                                    MethodAccessFlags::PUBLIC | MethodAccessFlags::PROTECTED,
                                )
                            })
                            .count(),
                    })
            })
            .collect();
        let internal_classes = classes
            .iter()
            .filter_map(|class| {
                let name = class.class_name().ok()?.to_string();
                name.starts_with("dev/mantle/internal/").then_some(name)
            })
            .collect();
        let manifest = EmissionManifest {
            schema_version: 1,
            expected_native_abi: expected_abi,
            reference_shells,
            internal_classes,
        };
        fs::write(manifest_output, serde_json::to_vec_pretty(&manifest)?)?;
    }
    Ok(())
}

fn internal_classes(expected_abi: u8) -> Result<Vec<ClassFile<'static>>> {
    Ok(vec![
        native_class(expected_abi)?,
        native_state_class()?,
        native_cleaner_class()?,
        native_probe_class()?,
        native_invocation_handler_class()?,
        native_load_future_class()?,
        native_load_callback_class()?,
        native_loader_class()?,
        native_audio_data_format_class()?,
        native_audio_frame_buffer_factory_class()?,
        native_event_dispatcher_class()?,
        native_audio_player_lifecycle_class()?,
    ])
}

pub fn emit_reference_slice(reference_jar: &Path, output: &Path) -> Result<()> {
    let mut source = ZipArchive::new(File::open(reference_jar)?)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut jar = ZipWriter::new(File::create(output)?);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    jar.start_file("META-INF/MANIFEST.MF", options)?;
    jar.write_all(b"Manifest-Version: 1.0\r\nCreated-By: Mantle Gate A reference slicer\r\n\r\n")?;
    for binary_name in REFERENCE_CLASSES {
        let entry_name = format!("{binary_name}.class");
        let mut entry = source.by_name(&entry_name)?;
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        jar.start_file(entry_name, options)?;
        jar.write_all(&bytes)?;
    }
    jar.finish()?;
    Ok(())
}

pub fn verify_structure(reference_jar: &Path, candidate_jar: &Path) -> Result<()> {
    let mut reference = ZipArchive::new(File::open(reference_jar)?)?;
    let mut candidate = ZipArchive::new(File::open(candidate_jar)?)?;
    for binary_name in REFERENCE_CLASSES {
        let entry_name = format!("{binary_name}.class");
        let reference_class = read_class(&mut reference, &entry_name)?;
        let candidate_class = read_class(&mut candidate, &entry_name)?;
        if reference_class.version != candidate_class.version
            || reference_class.access_flags != candidate_class.access_flags
            || reference_class.class_name()? != candidate_class.class_name()?
            || superclass_name(&reference_class)? != superclass_name(&candidate_class)?
            || interface_names(&reference_class)? != interface_names(&candidate_class)?
            || format!("{:?}", reference_class.attributes)
                != format!("{:?}", candidate_class.attributes)
        {
            return Err(format!("class structure differs for {binary_name}").into());
        }

        for field in reference_class.fields.iter().filter(|field| {
            field
                .access_flags
                .intersects(FieldAccessFlags::PUBLIC | FieldAccessFlags::PROTECTED)
        }) {
            let key = field_key(&reference_class, field)?;
            let Some(other) = candidate_class.fields.iter().find(|candidate| {
                field_key(&candidate_class, candidate).is_ok_and(|value| value == key)
            }) else {
                return Err(format!(
                    "candidate is missing field {binary_name}.{}{}",
                    key.0, key.1
                )
                .into());
            };
            if field.access_flags != other.access_flags
                || format!("{:?}", field.attributes) != format!("{:?}", other.attributes)
            {
                return Err(format!(
                    "field metadata differs for {binary_name}.{}{}",
                    key.0, key.1
                )
                .into());
            }
        }

        for method in reference_class.methods.iter().filter(|method| {
            method
                .access_flags
                .intersects(MethodAccessFlags::PUBLIC | MethodAccessFlags::PROTECTED)
        }) {
            let key = method_key(&reference_class, method)?;
            let Some(other) = candidate_class.methods.iter().find(|candidate| {
                method_key(&candidate_class, candidate).is_ok_and(|value| value == key)
            }) else {
                return Err(format!(
                    "candidate is missing method {binary_name}.{}{}",
                    key.0, key.1
                )
                .into());
            };
            let reference_attributes = method
                .attributes
                .iter()
                .filter(|attribute| !matches!(attribute, Attribute::Code { .. }))
                .collect::<Vec<_>>();
            let candidate_attributes = other
                .attributes
                .iter()
                .filter(|attribute| !matches!(attribute, Attribute::Code { .. }))
                .collect::<Vec<_>>();
            if method.access_flags != other.access_flags
                || format!("{reference_attributes:?}") != format!("{candidate_attributes:?}")
            {
                return Err(format!(
                    "method metadata differs for {binary_name}.{}{}",
                    key.0, key.1
                )
                .into());
            }
        }
    }
    Ok(())
}

fn read_class(archive: &mut ZipArchive<File>, entry_name: &str) -> Result<ClassFile<'static>> {
    let mut entry = archive.by_name(entry_name)?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes)?;
    Ok(ClassFile::from_bytes(&bytes)?)
}

fn interface_names(class: &ClassFile<'_>) -> Result<Vec<String>> {
    class
        .interfaces
        .iter()
        .map(|index| Ok(class.constant_pool.try_get_class(*index)?.to_string()))
        .collect()
}

fn superclass_name(class: &ClassFile<'_>) -> Result<Option<String>> {
    if class.super_class == 0 {
        Ok(None)
    } else {
        Ok(Some(
            class
                .constant_pool
                .try_get_class(class.super_class)?
                .to_string(),
        ))
    }
}

fn field_key(class: &ClassFile<'_>, field: &Field) -> Result<(String, String)> {
    Ok((
        class
            .constant_pool
            .try_get_utf8(field.name_index)?
            .to_string(),
        class
            .constant_pool
            .try_get_utf8(field.descriptor_index)?
            .to_string(),
    ))
}

fn method_key(class: &ClassFile<'_>, method: &Method) -> Result<(String, String)> {
    Ok((
        class
            .constant_pool
            .try_get_utf8(method.name_index)?
            .to_string(),
        class
            .constant_pool
            .try_get_utf8(method.descriptor_index)?
            .to_string(),
    ))
}

fn transform_reference_class(mut class: ClassFile<'static>) -> Result<ClassFile<'static>> {
    let class_name = class.class_name()?.to_string();
    class.fields.retain(|field| {
        matches!(
            class_name.as_str(),
            PLAYER_LIFECYCLE_MANAGER_CLASS | FUNCTIONAL_RESULT_HANDLER_CLASS
        ) || field
            .access_flags
            .intersects(FieldAccessFlags::PUBLIC | FieldAccessFlags::PROTECTED)
    });
    class.methods.retain(|method| {
        method
            .access_flags
            .intersects(MethodAccessFlags::PUBLIC | MethodAccessFlags::PROTECTED)
    });

    let pool = &mut class.constant_pool;
    for method in &mut class.methods {
        let name = pool.try_get_utf8(method.name_index)?.to_string();
        let descriptor_value = pool.try_get_utf8(method.descriptor_index)?;
        let (parameters, _) = FieldType::parse_method_descriptor(descriptor_value)?;
        let descriptor = descriptor_value.to_string();
        let parameter_slots = parameters
            .iter()
            .map(FieldType::slot_count)
            .map(u16::from)
            .sum::<u16>();
        let required_locals =
            parameter_slots + u16::from(!method.access_flags.contains(MethodAccessFlags::STATIC));
        let had_code = method
            .attributes
            .iter()
            .any(|attribute| matches!(attribute, Attribute::Code { .. }));
        method
            .attributes
            .retain(|attribute| !matches!(attribute, Attribute::Code { .. }));
        if !had_code {
            continue;
        }
        let body = replacement_body(pool, &class_name, &name, &descriptor, required_locals)?;
        method.attributes.push(body);
    }
    add_reference_implementation_state(&mut class, &class_name)?;
    Ok(class)
}

fn replacement_body(
    pool: &mut ConstantPool<'static>,
    class_name: &str,
    name: &str,
    descriptor: &str,
    required_locals: u16,
) -> Result<Attribute> {
    if class_name == MANAGER_CLASS {
        return manager_replacement(pool, name, descriptor, required_locals);
    }
    if class_name == AUDIO_REFERENCE_CLASS {
        return audio_reference_replacement(pool, name, descriptor, required_locals);
    }
    if class_name == DECODED_TRACK_HOLDER_CLASS {
        return decoded_track_holder_replacement(pool, name, descriptor, required_locals);
    }
    if class_name == BASIC_PLAYLIST_CLASS {
        return basic_playlist_replacement(pool, name, descriptor, required_locals);
    }
    if class_name == CONFIGURATION_CLASS {
        return audio_configuration_replacement(pool, name, descriptor, required_locals);
    }
    if class_name == AUDIO_PLAYER_OPTIONS_CLASS {
        return audio_player_options_replacement(pool, name, descriptor, required_locals);
    }
    if class_name == PLAYER_LIFECYCLE_MANAGER_CLASS {
        return audio_player_lifecycle_replacement(pool, name, descriptor, required_locals);
    }
    if class_name == FUNCTIONAL_RESULT_HANDLER_CLASS {
        return functional_result_handler_replacement(pool, name, descriptor, required_locals);
    }
    if track_enum_constants(class_name).is_some() {
        return track_enum_replacement(pool, class_name, name, descriptor, required_locals);
    }
    if matches!(
        class_name,
        ABSTRACT_MUTABLE_FRAME_CLASS | IMMUTABLE_FRAME_CLASS | MUTABLE_FRAME_CLASS
    ) {
        return audio_frame_replacement(pool, class_name, name, descriptor, required_locals);
    }
    if class_name == REFERENCE_MUTABLE_FRAME_CLASS {
        return reference_mutable_frame_replacement(pool, name, descriptor, required_locals);
    }
    if class_name == AUDIO_FRAME_PROVIDER_TOOLS_CLASS {
        return audio_frame_provider_tools_replacement(pool, name, descriptor, required_locals);
    }
    if class_name == AUDIO_PROCESSING_CONTEXT_CLASS {
        return audio_processing_context_replacement(pool, name, descriptor, required_locals);
    }
    if class_name == TERMINATOR_FRAME_CLASS {
        return terminator_frame_replacement(pool, name, descriptor, required_locals);
    }
    if class_name.starts_with("com/sedmelluq/discord/lavaplayer/player/event/") {
        return event_replacement(pool, class_name, name, descriptor, required_locals);
    }
    Ok(match (class_name, name, descriptor) {
        (
            "com/sedmelluq/discord/lavaplayer/track/AudioTrackInfo",
            "<init>",
            "(Ljava/lang/String;Ljava/lang/String;JLjava/lang/String;ZLjava/lang/String;Ljava/lang/String;Ljava/lang/String;)V",
        ) => track_info_constructor(pool)?,
        (
            "com/sedmelluq/discord/lavaplayer/track/AudioTrackInfo",
            "<init>",
            "(Ljava/lang/String;Ljava/lang/String;JLjava/lang/String;ZLjava/lang/String;)V",
        ) => track_info_short_constructor(pool)?,
        ("com/sedmelluq/discord/lavaplayer/format/AudioDataFormat", "<init>", "(III)V") => {
            audio_data_format_constructor(pool)?
        }
        (
            "com/sedmelluq/discord/lavaplayer/track/TrackMarker",
            "<init>",
            "(JLcom/sedmelluq/discord/lavaplayer/track/TrackMarkerHandler;)V",
        ) => track_marker_constructor(pool)?,
        _ => unsupported_body(
            pool,
            &format!("Gate A does not implement {class_name}.{name}{descriptor}"),
            required_locals,
        )?,
    })
}

fn audio_frame_provider_tools_replacement(
    pool: &mut ConstantPool<'static>,
    name: &str,
    descriptor: &str,
    required_locals: u16,
) -> Result<Attribute> {
    match (name, descriptor) {
        ("<init>", "()V") => object_constructor(pool),
        (
            "delegateToTimedProvide",
            "(Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioFrameProvider;)Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioFrame;",
        ) => delegate_to_timed_provide(pool),
        _ => unsupported_body(
            pool,
            &format!(
                "Phase 13 does not implement {AUDIO_FRAME_PROVIDER_TOOLS_CLASS}.{name}{descriptor}"
            ),
            required_locals,
        ),
    }
}

fn decoded_track_holder_replacement(
    pool: &mut ConstantPool<'static>,
    name: &str,
    descriptor: &str,
    required_locals: u16,
) -> Result<Attribute> {
    match (name, descriptor) {
        ("<init>", "(Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;)V") => {
            decoded_track_holder_constructor(pool)
        }
        _ => unsupported_body(
            pool,
            &format!("Phase 13 does not implement {DECODED_TRACK_HOLDER_CLASS}.{name}{descriptor}"),
            required_locals,
        ),
    }
}

fn audio_player_options_replacement(
    pool: &mut ConstantPool<'static>,
    name: &str,
    descriptor: &str,
    required_locals: u16,
) -> Result<Attribute> {
    match (name, descriptor) {
        ("<init>", "()V") => audio_player_options_constructor(pool),
        _ => unsupported_body(
            pool,
            &format!("Phase 13 does not implement {AUDIO_PLAYER_OPTIONS_CLASS}.{name}{descriptor}"),
            required_locals,
        ),
    }
}

fn audio_player_lifecycle_replacement(
    pool: &mut ConstantPool<'static>,
    name: &str,
    descriptor: &str,
    required_locals: u16,
) -> Result<Attribute> {
    match (name, descriptor) {
        (
            "<init>",
            "(Ljava/util/concurrent/ScheduledExecutorService;Ljava/util/concurrent/atomic/AtomicLong;)V",
        ) => audio_player_lifecycle_constructor(pool),
        ("initialise", "()V") => audio_player_lifecycle_initialise(pool),
        ("shutdown", "()V") => audio_player_lifecycle_shutdown(pool),
        ("onEvent", "(Lcom/sedmelluq/discord/lavaplayer/player/event/AudioEvent;)V") => {
            audio_player_lifecycle_on_event(pool)
        }
        ("run", "()V") => audio_player_lifecycle_run(pool),
        _ => unsupported_body(
            pool,
            &format!(
                "Phase 13 does not implement {PLAYER_LIFECYCLE_MANAGER_CLASS}.{name}{descriptor}"
            ),
            required_locals,
        ),
    }
}

fn functional_result_handler_replacement(
    pool: &mut ConstantPool<'static>,
    name: &str,
    descriptor: &str,
    required_locals: u16,
) -> Result<Attribute> {
    match (name, descriptor) {
        (
            "<init>",
            "(Ljava/util/function/Consumer;Ljava/util/function/Consumer;Ljava/lang/Runnable;Ljava/util/function/Consumer;)V",
        ) => functional_result_handler_constructor(pool),
        ("trackLoaded", "(Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;)V") => {
            functional_result_handler_consumer(
                pool,
                "trackConsumer",
                "Ljava/util/function/Consumer;",
            )
        }
        ("playlistLoaded", "(Lcom/sedmelluq/discord/lavaplayer/track/AudioPlaylist;)V") => {
            functional_result_handler_consumer(
                pool,
                "playlistConsumer",
                "Ljava/util/function/Consumer;",
            )
        }
        ("noMatches", "()V") => functional_result_handler_runnable(pool),
        ("loadFailed", "(Lcom/sedmelluq/discord/lavaplayer/tools/FriendlyException;)V") => {
            functional_result_handler_consumer(
                pool,
                "exceptionConsumer",
                "Ljava/util/function/Consumer;",
            )
        }
        _ => unsupported_body(
            pool,
            &format!(
                "Phase 13 does not implement {FUNCTIONAL_RESULT_HANDLER_CLASS}.{name}{descriptor}"
            ),
            required_locals,
        ),
    }
}

fn audio_processing_context_replacement(
    pool: &mut ConstantPool<'static>,
    name: &str,
    descriptor: &str,
    required_locals: u16,
) -> Result<Attribute> {
    match (name, descriptor) {
        (
            "<init>",
            "(Lcom/sedmelluq/discord/lavaplayer/player/AudioConfiguration;Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioFrameBuffer;Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayerOptions;Lcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;)V",
        ) => audio_processing_context_constructor(pool),
        _ => unsupported_body(
            pool,
            &format!(
                "Phase 13 does not implement {AUDIO_PROCESSING_CONTEXT_CLASS}.{name}{descriptor}"
            ),
            required_locals,
        ),
    }
}

fn terminator_frame_replacement(
    pool: &mut ConstantPool<'static>,
    name: &str,
    descriptor: &str,
    required_locals: u16,
) -> Result<Attribute> {
    match (name, descriptor) {
        ("<init>", "()V") => object_constructor(pool),
        ("isTerminator", "()Z") => boolean_return(pool, true, required_locals),
        _ => unsupported_without_message(pool, required_locals),
    }
}

fn track_enum_constants(class_name: &str) -> Option<&'static [&'static str]> {
    match class_name {
        TRACK_END_REASON_CLASS => {
            Some(&["FINISHED", "LOAD_FAILED", "STOPPED", "REPLACED", "CLEANUP"])
        }
        TRACK_STATE_CLASS => Some(&[
            "INACTIVE", "LOADING", "PLAYING", "SEEKING", "STOPPING", "FINISHED",
        ]),
        MARKER_STATE_CLASS => Some(&[
            "REACHED",
            "REMOVED",
            "OVERWRITTEN",
            "BYPASSED",
            "STOPPED",
            "LATE",
            "ENDED",
        ]),
        RESAMPLING_CLASS => Some(&["HIGH", "MEDIUM", "LOW"]),
        _ => None,
    }
}

fn audio_configuration_replacement(
    pool: &mut ConstantPool<'static>,
    name: &str,
    descriptor: &str,
    required_locals: u16,
) -> Result<Attribute> {
    let body = match (name, descriptor) {
        ("<init>", "()V") => audio_configuration_constructor(pool)?,
        (
            "getResamplingQuality",
            "()Lcom/sedmelluq/discord/lavaplayer/player/AudioConfiguration$ResamplingQuality;",
        ) => object_getter(
            pool,
            CONFIGURATION_CLASS,
            "resamplingQuality",
            "Lcom/sedmelluq/discord/lavaplayer/player/AudioConfiguration$ResamplingQuality;",
        )?,
        (
            "setResamplingQuality",
            "(Lcom/sedmelluq/discord/lavaplayer/player/AudioConfiguration$ResamplingQuality;)V",
        ) => object_setter(
            pool,
            CONFIGURATION_CLASS,
            "resamplingQuality",
            "Lcom/sedmelluq/discord/lavaplayer/player/AudioConfiguration$ResamplingQuality;",
        )?,
        ("getOpusEncodingQuality", "()I") => {
            int_getter(pool, CONFIGURATION_CLASS, "opusEncodingQuality")?
        }
        ("setOpusEncodingQuality", "(I)V") => audio_configuration_set_opus_quality(pool)?,
        ("getOutputFormat", "()Lcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;") => {
            object_getter(
                pool,
                CONFIGURATION_CLASS,
                "outputFormat",
                "Lcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;",
            )?
        }
        ("setOutputFormat", "(Lcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;)V") => {
            object_setter(
                pool,
                CONFIGURATION_CLASS,
                "outputFormat",
                "Lcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;",
            )?
        }
        ("isFilterHotSwapEnabled", "()Z") => {
            bool_getter(pool, CONFIGURATION_CLASS, "filterHotSwapEnabled")?
        }
        ("setFilterHotSwapEnabled", "(Z)V") => {
            bool_setter(pool, CONFIGURATION_CLASS, "filterHotSwapEnabled")?
        }
        (
            "getFrameBufferFactory",
            "()Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioFrameBufferFactory;",
        ) => object_getter(
            pool,
            CONFIGURATION_CLASS,
            "frameBufferFactory",
            "Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioFrameBufferFactory;",
        )?,
        (
            "setFrameBufferFactory",
            "(Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioFrameBufferFactory;)V",
        ) => object_setter(
            pool,
            CONFIGURATION_CLASS,
            "frameBufferFactory",
            "Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioFrameBufferFactory;",
        )?,
        ("copy", "()Lcom/sedmelluq/discord/lavaplayer/player/AudioConfiguration;") => {
            audio_configuration_copy(pool)?
        }
        _ => unsupported_body(
            pool,
            &format!("Phase 13 does not implement {CONFIGURATION_CLASS}.{name}{descriptor}"),
            required_locals,
        )?,
    };
    Ok(body)
}

fn audio_frame_replacement(
    pool: &mut ConstantPool<'static>,
    class_name: &str,
    name: &str,
    descriptor: &str,
    required_locals: u16,
) -> Result<Attribute> {
    let body = match (class_name, name, descriptor) {
        (ABSTRACT_MUTABLE_FRAME_CLASS, "<init>", "()V") => object_constructor(pool)?,
        (ABSTRACT_MUTABLE_FRAME_CLASS, "getTimecode", "()J") => {
            long_getter(pool, ABSTRACT_MUTABLE_FRAME_CLASS, "timecode")?
        }
        (ABSTRACT_MUTABLE_FRAME_CLASS, "setTimecode", "(J)V") => {
            long_setter(pool, ABSTRACT_MUTABLE_FRAME_CLASS, "timecode")?
        }
        (ABSTRACT_MUTABLE_FRAME_CLASS, "getVolume", "()I") => {
            int_getter(pool, ABSTRACT_MUTABLE_FRAME_CLASS, "volume")?
        }
        (ABSTRACT_MUTABLE_FRAME_CLASS, "setVolume", "(I)V") => {
            int_setter(pool, ABSTRACT_MUTABLE_FRAME_CLASS, "volume")?
        }
        (
            ABSTRACT_MUTABLE_FRAME_CLASS,
            "getFormat",
            "()Lcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;",
        ) => object_getter(
            pool,
            ABSTRACT_MUTABLE_FRAME_CLASS,
            "format",
            "Lcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;",
        )?,
        (
            ABSTRACT_MUTABLE_FRAME_CLASS,
            "setFormat",
            "(Lcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;)V",
        ) => object_setter(
            pool,
            ABSTRACT_MUTABLE_FRAME_CLASS,
            "format",
            "Lcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;",
        )?,
        (ABSTRACT_MUTABLE_FRAME_CLASS, "isTerminator", "()Z") => {
            bool_getter(pool, ABSTRACT_MUTABLE_FRAME_CLASS, "terminator")?
        }
        (ABSTRACT_MUTABLE_FRAME_CLASS, "setTerminator", "(Z)V") => {
            bool_setter(pool, ABSTRACT_MUTABLE_FRAME_CLASS, "terminator")?
        }
        (
            ABSTRACT_MUTABLE_FRAME_CLASS,
            "freeze",
            "()Lcom/sedmelluq/discord/lavaplayer/track/playback/ImmutableAudioFrame;",
        ) => mutable_frame_freeze(pool)?,
        (
            IMMUTABLE_FRAME_CLASS,
            "<init>",
            "(J[BILcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;)V",
        ) => immutable_frame_constructor(pool)?,
        (IMMUTABLE_FRAME_CLASS, "getTimecode", "()J") => {
            long_getter(pool, IMMUTABLE_FRAME_CLASS, "timecode")?
        }
        (IMMUTABLE_FRAME_CLASS, "getVolume", "()I") => {
            int_getter(pool, IMMUTABLE_FRAME_CLASS, "volume")?
        }
        (IMMUTABLE_FRAME_CLASS, "getDataLength", "()I") => immutable_frame_data_length(pool)?,
        (IMMUTABLE_FRAME_CLASS, "getData", "()[B") => {
            object_getter(pool, IMMUTABLE_FRAME_CLASS, "data", "[B")?
        }
        (IMMUTABLE_FRAME_CLASS, "getData", "([BI)V") => immutable_frame_copy_data(pool)?,
        (
            IMMUTABLE_FRAME_CLASS,
            "getFormat",
            "()Lcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;",
        ) => object_getter(
            pool,
            IMMUTABLE_FRAME_CLASS,
            "format",
            "Lcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;",
        )?,
        (IMMUTABLE_FRAME_CLASS, "isTerminator", "()Z") => boolean_return(pool, false, 1)?,
        (MUTABLE_FRAME_CLASS, "<init>", "()V") => mutable_frame_constructor(pool, false)?,
        (MUTABLE_FRAME_CLASS, "<init>", "(Ljava/nio/ByteBuffer;)V") => {
            mutable_frame_constructor(pool, true)?
        }
        (MUTABLE_FRAME_CLASS, "setBuffer", "(Ljava/nio/ByteBuffer;)V") => {
            mutable_frame_set_buffer(pool)?
        }
        (MUTABLE_FRAME_CLASS, "store", "([BII)V") => mutable_frame_store(pool)?,
        (MUTABLE_FRAME_CLASS, "getDataLength", "()I") => {
            int_getter(pool, MUTABLE_FRAME_CLASS, "frameLength")?
        }
        (MUTABLE_FRAME_CLASS, "getData", "()[B") => mutable_frame_get_data(pool)?,
        (MUTABLE_FRAME_CLASS, "getData", "([BI)V") => mutable_frame_copy_data(pool)?,
        _ => unsupported_body(
            pool,
            &format!("Phase 13 does not implement {class_name}.{name}{descriptor}"),
            required_locals,
        )?,
    };
    Ok(body)
}

fn reference_mutable_frame_replacement(
    pool: &mut ConstantPool<'static>,
    name: &str,
    descriptor: &str,
    required_locals: u16,
) -> Result<Attribute> {
    let body = match (name, descriptor) {
        ("<init>", "()V") => mutable_frame_constructor(pool, false)?,
        ("getFrameBuffer", "()[B") => {
            object_getter(pool, REFERENCE_MUTABLE_FRAME_CLASS, "frameBuffer", "[B")?
        }
        ("getFrameOffset", "()I") => {
            int_getter(pool, REFERENCE_MUTABLE_FRAME_CLASS, "frameOffset")?
        }
        ("getFrameEndOffset", "()I") => reference_mutable_frame_end_offset(pool)?,
        ("getDataLength", "()I") => int_getter(pool, REFERENCE_MUTABLE_FRAME_CLASS, "frameLength")?,
        ("getData", "()[B") => frame_get_data_copy(pool, REFERENCE_MUTABLE_FRAME_CLASS)?,
        ("getData", "([BI)V") => reference_mutable_frame_copy_data(pool)?,
        ("setDataReference", "([BII)V") => reference_mutable_frame_set_data(pool)?,
        _ => unsupported_body(
            pool,
            &format!(
                "Phase 13 does not implement {REFERENCE_MUTABLE_FRAME_CLASS}.{name}{descriptor}"
            ),
            required_locals,
        )?,
    };
    Ok(body)
}

fn track_enum_replacement(
    pool: &mut ConstantPool<'static>,
    class_name: &str,
    name: &str,
    descriptor: &str,
    required_locals: u16,
) -> Result<Attribute> {
    let values_descriptor = format!("()[L{class_name};");
    let value_of_descriptor = format!("(Ljava/lang/String;)L{class_name};");
    match (name, descriptor) {
        ("values", value) if value == values_descriptor => track_enum_values(pool, class_name),
        ("valueOf", value) if value == value_of_descriptor => track_enum_value_of(pool, class_name),
        _ => unsupported_body(
            pool,
            &format!("Phase 13 does not implement {class_name}.{name}{descriptor}"),
            required_locals,
        ),
    }
}

fn track_enum_values(pool: &mut ConstantPool<'static>, class_name: &str) -> Result<Attribute> {
    let constants = track_enum_constants(class_name).ok_or("unknown track enum")?;
    let owner = pool.add_class(class_name)?;
    let descriptor = format!("L{class_name};");
    let mut instructions = vec![
        small_integer_instruction(constants.len())?,
        Instruction::Anewarray(owner),
    ];
    for (ordinal, name) in constants.iter().enumerate() {
        let field = pool.add_field_ref(owner, *name, &descriptor)?;
        instructions.extend([
            Instruction::Dup,
            small_integer_instruction(ordinal)?,
            Instruction::Getstatic(field),
            Instruction::Aastore,
        ]);
    }
    instructions.push(Instruction::Areturn);
    code(pool, 4, 0, instructions)
}

fn track_enum_value_of(pool: &mut ConstantPool<'static>, class_name: &str) -> Result<Attribute> {
    let owner = pool.add_class(class_name)?;
    let enumeration = pool.add_class("java/lang/Enum")?;
    let value_of = pool.add_method_ref(
        enumeration,
        "valueOf",
        "(Ljava/lang/Class;Ljava/lang/String;)Ljava/lang/Enum;",
    )?;
    code(
        pool,
        2,
        1,
        vec![
            Instruction::Ldc_w(owner),
            Instruction::Aload_0,
            Instruction::Invokestatic(value_of),
            Instruction::Checkcast(owner),
            Instruction::Areturn,
        ],
    )
}

fn small_integer_instruction(value: usize) -> Result<Instruction> {
    Ok(match value {
        0 => Instruction::Iconst_0,
        1 => Instruction::Iconst_1,
        2 => Instruction::Iconst_2,
        3 => Instruction::Iconst_3,
        4 => Instruction::Iconst_4,
        5 => Instruction::Iconst_5,
        value => Instruction::Bipush(i8::try_from(value)?),
    })
}

fn event_replacement(
    pool: &mut ConstantPool<'static>,
    class_name: &str,
    name: &str,
    descriptor: &str,
    required_locals: u16,
) -> Result<Attribute> {
    if class_name == EVENT_ADAPTER_CLASS && event_adapter_method_is_noop(name, descriptor) {
        return void_return(pool, required_locals);
    }
    Ok(match (class_name, name, descriptor) {
        (
            "com/sedmelluq/discord/lavaplayer/player/event/AudioEvent",
            "<init>",
            "(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;)V",
        ) => audio_event_constructor(pool)?,
        (
            "com/sedmelluq/discord/lavaplayer/player/event/TrackStartEvent",
            "<init>",
            "(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;)V",
        ) => track_start_constructor(pool)?,
        (
            "com/sedmelluq/discord/lavaplayer/player/event/TrackEndEvent",
            "<init>",
            "(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackEndReason;)V",
        ) => track_end_constructor(pool)?,
        (
            "com/sedmelluq/discord/lavaplayer/player/event/PlayerPauseEvent"
            | "com/sedmelluq/discord/lavaplayer/player/event/PlayerResumeEvent",
            "<init>",
            "(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;)V",
        ) => simple_audio_event_constructor(pool)?,
        (EVENT_ADAPTER_CLASS, "<init>", "()V") => object_constructor(pool)?,
        (
            EVENT_ADAPTER_CLASS,
            "onTrackStuck",
            "(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;J[Ljava/lang/StackTraceElement;)V",
        ) => event_adapter_stuck_with_trace(pool)?,
        (
            EVENT_ADAPTER_CLASS,
            "onEvent",
            "(Lcom/sedmelluq/discord/lavaplayer/player/event/AudioEvent;)V",
        ) => event_adapter_dispatch(pool)?,
        (
            TRACK_EXCEPTION_EVENT_CLASS,
            "<init>",
            "(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;Lcom/sedmelluq/discord/lavaplayer/tools/FriendlyException;)V",
        ) => track_exception_constructor(pool)?,
        (
            TRACK_STUCK_EVENT_CLASS,
            "<init>",
            "(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;J[Ljava/lang/StackTraceElement;)V",
        ) => track_stuck_constructor(pool)?,
        _ => unsupported_body(
            pool,
            &format!("Phase 13 does not implement {class_name}.{name}{descriptor}"),
            required_locals,
        )?,
    })
}

fn event_adapter_method_is_noop(name: &str, descriptor: &str) -> bool {
    matches!(
        (name, descriptor),
        (
            "onPlayerPause" | "onPlayerResume",
            "(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;)V"
        ) | (
            "onTrackStart",
            "(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;)V"
        ) | (
            "onTrackEnd",
            "(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackEndReason;)V"
        ) | (
            "onTrackException",
            "(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;Lcom/sedmelluq/discord/lavaplayer/tools/FriendlyException;)V"
        ) | (
            "onTrackStuck",
            "(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;J)V"
        )
    )
}

fn manager_replacement(
    pool: &mut ConstantPool<'static>,
    name: &str,
    descriptor: &str,
    required_locals: u16,
) -> Result<Attribute> {
    Ok(match (name, descriptor) {
        ("<init>", "()V") => manager_constructor(pool)?,
        ("shutdown", "()V") => manager_shutdown(pool)?,
        ("getConfiguration", "()Lcom/sedmelluq/discord/lavaplayer/player/AudioConfiguration;") => {
            manager_configuration(pool)?
        }
        ("getFrameBufferDuration", "()I") => integer_return(pool, 5_000, 1)?,
        ("isUsingSeekGhosting", "()Z") => boolean_return(pool, true, 1)?,
        (
            "createPlayer" | "constructPlayer",
            "()Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;",
        ) => manager_create_player(pool)?,
        ("encodeTrackDetails", "(Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;)[B") => {
            manager_encode_track_details(pool)?
        }
        (
            "decodeTrackDetails",
            "(Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackInfo;[B)Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;",
        ) => manager_decode_track_details(pool)?,
        (
            "registerSourceManager",
            "(Lcom/sedmelluq/discord/lavaplayer/source/AudioSourceManager;)V",
        ) => manager_register_source(pool)?,
        (
            "source",
            "(Ljava/lang/Class;)Lcom/sedmelluq/discord/lavaplayer/source/AudioSourceManager;",
        ) => manager_source(pool)?,
        ("getSourceManagers", "()Ljava/util/List;") => manager_get_sources(pool)?,
        (
            "loadItemSync",
            "(Lcom/sedmelluq/discord/lavaplayer/track/AudioReference;)Lcom/sedmelluq/discord/lavaplayer/track/AudioItem;",
        ) => manager_load_sync(pool)?,
        (
            "loadItemSync",
            "(Lcom/sedmelluq/discord/lavaplayer/track/AudioReference;Lcom/sedmelluq/discord/lavaplayer/player/AudioLoadResultHandler;)V",
        ) => manager_load_sync_handled(pool)?,
        (
            "loadItem",
            "(Lcom/sedmelluq/discord/lavaplayer/track/AudioReference;Lcom/sedmelluq/discord/lavaplayer/player/AudioLoadResultHandler;)Ljava/util/concurrent/Future;",
        ) => manager_load_reference(pool)?,
        (
            "loadItemOrdered",
            "(Ljava/lang/Object;Lcom/sedmelluq/discord/lavaplayer/track/AudioReference;Lcom/sedmelluq/discord/lavaplayer/player/AudioLoadResultHandler;)Ljava/util/concurrent/Future;",
        ) => manager_load_ordered_reference(pool)?,
        _ => unsupported_body(
            pool,
            &format!("Gate A does not implement {MANAGER_CLASS}.{name}{descriptor}"),
            required_locals,
        )?,
    })
}

fn audio_reference_replacement(
    pool: &mut ConstantPool<'static>,
    name: &str,
    descriptor: &str,
    required_locals: u16,
) -> Result<Attribute> {
    Ok(match (name, descriptor) {
        ("<init>", "(Ljava/lang/String;Ljava/lang/String;)V") => {
            audio_reference_short_constructor(pool)?
        }
        (
            "<init>",
            "(Ljava/lang/String;Ljava/lang/String;Lcom/sedmelluq/discord/lavaplayer/container/MediaContainerDescriptor;)V",
        ) => audio_reference_constructor(pool)?,
        ("getTitle", "()Ljava/lang/String;") => {
            object_getter(pool, AUDIO_REFERENCE_CLASS, "title", "Ljava/lang/String;")?
        }
        ("getIdentifier" | "getUri", "()Ljava/lang/String;") => object_getter(
            pool,
            AUDIO_REFERENCE_CLASS,
            "identifier",
            "Ljava/lang/String;",
        )?,
        ("getAuthor" | "getLength" | "getArtworkUrl" | "getISRC", _) => null_return(pool, 1)?,
        ("<clinit>", "()V") => audio_reference_clinit(pool)?,
        _ => unsupported_body(
            pool,
            &format!("Gate A does not implement {AUDIO_REFERENCE_CLASS}.{name}{descriptor}"),
            required_locals,
        )?,
    })
}

fn basic_playlist_replacement(
    pool: &mut ConstantPool<'static>,
    name: &str,
    descriptor: &str,
    required_locals: u16,
) -> Result<Attribute> {
    Ok(match (name, descriptor) {
        (
            "<init>",
            "(Ljava/lang/String;Ljava/util/List;Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;Z)V",
        ) => basic_playlist_constructor(pool)?,
        ("getName", "()Ljava/lang/String;") => object_getter(
            pool,
            BASIC_PLAYLIST_CLASS,
            "mantleName",
            "Ljava/lang/String;",
        )?,
        ("getTracks", "()Ljava/util/List;") => object_getter(
            pool,
            BASIC_PLAYLIST_CLASS,
            "mantleTracks",
            "Ljava/util/List;",
        )?,
        ("getSelectedTrack", "()Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;") => {
            object_getter(
                pool,
                BASIC_PLAYLIST_CLASS,
                "mantleSelectedTrack",
                "Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;",
            )?
        }
        ("isSearchResult", "()Z") => bool_getter(pool, BASIC_PLAYLIST_CLASS, "mantleSearchResult")?,
        _ => unsupported_body(
            pool,
            &format!("Gate A does not implement {BASIC_PLAYLIST_CLASS}.{name}{descriptor}"),
            required_locals,
        )?,
    })
}

fn add_reference_implementation_state(
    class: &mut ClassFile<'static>,
    class_name: &str,
) -> Result<()> {
    if class_name == AUDIO_REFERENCE_CLASS {
        add_audio_reference_state(class)?;
    }
    if class_name == TERMINATOR_FRAME_CLASS {
        add_terminator_frame_state(class)?;
    }
    if track_enum_constants(class_name).is_some() {
        add_track_enum_state(class, class_name)?;
    }
    add_audio_frame_implementation_state(class, class_name)?;
    if class_name == CONFIGURATION_CLASS {
        for (name, descriptor) in [
            (
                "resamplingQuality",
                "Lcom/sedmelluq/discord/lavaplayer/player/AudioConfiguration$ResamplingQuality;",
            ),
            ("opusEncodingQuality", "I"),
            (
                "outputFormat",
                "Lcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;",
            ),
            ("filterHotSwapEnabled", "Z"),
            (
                "frameBufferFactory",
                "Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioFrameBufferFactory;",
            ),
        ] {
            add_field(
                class,
                FieldAccessFlags::PRIVATE | FieldAccessFlags::VOLATILE,
                name,
                descriptor,
            )?;
        }
    }
    if class_name == MANAGER_CLASS {
        add_field(
            class,
            FieldAccessFlags::PRIVATE | FieldAccessFlags::FINAL,
            "mantleHandle",
            "J",
        )?;
        add_field(
            class,
            FieldAccessFlags::PRIVATE | FieldAccessFlags::FINAL,
            "mantleCleanable",
            "Ljava/lang/ref/Cleaner$Cleanable;",
        )?;
        add_field(
            class,
            FieldAccessFlags::PRIVATE | FieldAccessFlags::FINAL,
            "mantleSources",
            "Ljava/util/ArrayList;",
        )?;
        let body = manager_load_string(&mut class.constant_pool)?;
        add_method(
            class,
            MethodAccessFlags::PUBLIC,
            "loadItem",
            "(Ljava/lang/String;Lcom/sedmelluq/discord/lavaplayer/player/AudioLoadResultHandler;)Ljava/util/concurrent/Future;",
            Some(body),
        )?;
        let body = manager_load_ordered_string(&mut class.constant_pool)?;
        add_method(
            class,
            MethodAccessFlags::PUBLIC,
            "loadItemOrdered",
            "(Ljava/lang/Object;Ljava/lang/String;Lcom/sedmelluq/discord/lavaplayer/player/AudioLoadResultHandler;)Ljava/util/concurrent/Future;",
            Some(body),
        )?;
    }
    if class_name == BASIC_PLAYLIST_CLASS {
        add_basic_playlist_state(class)?;
    }
    Ok(())
}

fn add_audio_frame_implementation_state(
    class: &mut ClassFile<'static>,
    class_name: &str,
) -> Result<()> {
    let fields: &[(&str, &str)] = match class_name {
        ABSTRACT_MUTABLE_FRAME_CLASS => &[
            ("timecode", "J"),
            ("volume", "I"),
            (
                "format",
                "Lcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;",
            ),
            ("terminator", "Z"),
        ],
        MUTABLE_FRAME_CLASS => &[
            ("frameBuffer", "Ljava/nio/ByteBuffer;"),
            ("framePosition", "I"),
            ("frameLength", "I"),
        ],
        REFERENCE_MUTABLE_FRAME_CLASS => &[
            ("frameBuffer", "[B"),
            ("frameOffset", "I"),
            ("frameLength", "I"),
        ],
        _ => &[],
    };
    for (name, descriptor) in fields {
        add_field(class, FieldAccessFlags::PRIVATE, name, descriptor)?;
    }
    Ok(())
}

fn add_audio_reference_state(class: &mut ClassFile<'static>) -> Result<()> {
    let body = audio_reference_clinit(&mut class.constant_pool)?;
    add_method(
        class,
        MethodAccessFlags::STATIC,
        "<clinit>",
        "()V",
        Some(body),
    )
}

fn add_terminator_frame_state(class: &mut ClassFile<'static>) -> Result<()> {
    let body = terminator_frame_clinit(&mut class.constant_pool)?;
    add_method(
        class,
        MethodAccessFlags::STATIC,
        "<clinit>",
        "()V",
        Some(body),
    )
}

fn add_track_enum_state(class: &mut ClassFile<'static>, class_name: &str) -> Result<()> {
    let constructor_descriptor = if class_name == TRACK_END_REASON_CLASS {
        "(Ljava/lang/String;IZ)V"
    } else {
        "(Ljava/lang/String;I)V"
    };
    let constructor = if class_name == TRACK_END_REASON_CLASS {
        end_reason_constructor(&mut class.constant_pool)?
    } else {
        enum_constructor(&mut class.constant_pool)?
    };
    add_method(
        class,
        MethodAccessFlags::PRIVATE,
        "<init>",
        constructor_descriptor,
        Some(constructor),
    )?;
    let initializer =
        track_enum_initializer(&mut class.constant_pool, class_name, constructor_descriptor)?;
    add_method(
        class,
        MethodAccessFlags::STATIC,
        "<clinit>",
        "()V",
        Some(initializer),
    )
}

fn track_enum_initializer(
    pool: &mut ConstantPool<'static>,
    class_name: &str,
    constructor_descriptor: &str,
) -> Result<Attribute> {
    let constants = track_enum_constants(class_name).ok_or("unknown track enum")?;
    let owner = pool.add_class(class_name)?;
    let constructor = pool.add_method_ref(owner, "<init>", constructor_descriptor)?;
    let descriptor = format!("L{class_name};");
    let mut instructions = Vec::with_capacity(constants.len() * 7 + 1);
    for (ordinal, name) in constants.iter().enumerate() {
        instructions.extend([
            Instruction::New(owner),
            Instruction::Dup,
            Instruction::Ldc_w(pool.add_string(*name)?),
            small_integer_instruction(ordinal)?,
        ]);
        if class_name == TRACK_END_REASON_CLASS {
            instructions.push(if ordinal < 2 {
                Instruction::Iconst_1
            } else {
                Instruction::Iconst_0
            });
        }
        let field = pool.add_field_ref(owner, *name, &descriptor)?;
        instructions.extend([
            Instruction::Invokespecial(constructor),
            Instruction::Putstatic(field),
        ]);
    }
    instructions.push(Instruction::Return);
    code(
        pool,
        if class_name == TRACK_END_REASON_CLASS {
            5
        } else {
            4
        },
        0,
        instructions,
    )
}

fn add_basic_playlist_state(class: &mut ClassFile<'static>) -> Result<()> {
    for (name, descriptor) in [
        ("mantleName", "Ljava/lang/String;"),
        ("mantleTracks", "Ljava/util/List;"),
        (
            "mantleSelectedTrack",
            "Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;",
        ),
        ("mantleSearchResult", "Z"),
    ] {
        add_field(
            class,
            FieldAccessFlags::PRIVATE | FieldAccessFlags::FINAL,
            name,
            descriptor,
        )?;
    }
    Ok(())
}

fn native_class(expected_abi: u8) -> Result<ClassFile<'static>> {
    let mut class = new_class(
        NATIVE_CLASS,
        "java/lang/Object",
        ClassAccessFlags::PUBLIC | ClassAccessFlags::FINAL | ClassAccessFlags::SUPER,
        &[],
    )?;
    let constructor = object_constructor(&mut class.constant_pool)?;
    add_method(
        &mut class,
        MethodAccessFlags::PRIVATE,
        "<init>",
        "()V",
        Some(constructor),
    )?;
    for (name, descriptor) in [
        ("abiVersion", "()I"),
        ("buildId", "()Ljava/lang/String;"),
        ("capabilities", "()J"),
        ("createHandle", "(I)J"),
        ("createCoreHandle", "(ILjava/lang/String;)J"),
        ("release", "(J)V"),
        ("liveHandles", "()I"),
        ("validateHandle", "(JI)Z"),
        ("identity", "(Ljava/lang/Object;)Ljava/lang/Object;"),
        ("dispatchOnCurrentThread", "(Ljava/lang/Runnable;I)V"),
        ("dispatchOnNativeDaemon", "(Ljava/lang/Runnable;I)Z"),
        ("callbackExceptions", "()I"),
        ("resetCallbackExceptions", "()V"),
        ("createProxy", "(ILjava/lang/String;)Ljava/lang/Object;"),
        (
            "loadItem",
            "(Ljava/lang/String;Lcom/sedmelluq/discord/lavaplayer/player/AudioLoadResultHandler;)Ljava/util/concurrent/Future;",
        ),
        (
            "loadItemOrdered",
            "(Ljava/lang/Object;Ljava/lang/String;Lcom/sedmelluq/discord/lavaplayer/player/AudioLoadResultHandler;)Ljava/util/concurrent/Future;",
        ),
        (
            "registerSourceManager",
            "(Lcom/sedmelluq/discord/lavaplayer/player/DefaultAudioPlayerManager;Lcom/sedmelluq/discord/lavaplayer/source/AudioSourceManager;)V",
        ),
        (
            "sourceManager",
            "(Lcom/sedmelluq/discord/lavaplayer/player/DefaultAudioPlayerManager;Ljava/lang/Class;)Lcom/sedmelluq/discord/lavaplayer/source/AudioSourceManager;",
        ),
        (
            "loadItemSync",
            "(Lcom/sedmelluq/discord/lavaplayer/track/AudioReference;)Lcom/sedmelluq/discord/lavaplayer/track/AudioItem;",
        ),
        (
            "loadItemSyncHandled",
            "(Lcom/sedmelluq/discord/lavaplayer/track/AudioReference;Lcom/sedmelluq/discord/lavaplayer/player/AudioLoadResultHandler;)V",
        ),
        (
            "loadItemReference",
            "(Lcom/sedmelluq/discord/lavaplayer/track/AudioReference;Lcom/sedmelluq/discord/lavaplayer/player/AudioLoadResultHandler;)Ljava/util/concurrent/Future;",
        ),
        (
            "loadItemOrderedReference",
            "(Ljava/lang/Object;Lcom/sedmelluq/discord/lavaplayer/track/AudioReference;Lcom/sedmelluq/discord/lavaplayer/player/AudioLoadResultHandler;)Ljava/util/concurrent/Future;",
        ),
        ("cancelLoad", "(JZ)Z"),
        ("dispatchLoad", "(J)V"),
        ("orderingKeyCount", "()I"),
        ("trackedSourceItemCount", "()I"),
        (
            "encodeTrackDetails",
            "(Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;)[B",
        ),
        (
            "decodeTrackDetails",
            "(Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackInfo;[B)Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;",
        ),
    ] {
        add_method(
            &mut class,
            MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC | MethodAccessFlags::NATIVE,
            name,
            descriptor,
            None,
        )?;
    }
    let body = ensure_abi_body(&mut class.constant_pool, expected_abi)?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
        "ensureAbi",
        "()V",
        Some(body),
    )?;
    Ok(class)
}

fn native_state_class() -> Result<ClassFile<'static>> {
    let mut class = new_class(
        STATE_CLASS,
        "java/lang/Object",
        ClassAccessFlags::PUBLIC | ClassAccessFlags::FINAL | ClassAccessFlags::SUPER,
        &["java/lang/Runnable"],
    )?;
    add_field(
        &mut class,
        FieldAccessFlags::PRIVATE | FieldAccessFlags::FINAL,
        "handle",
        "J",
    )?;
    let body = state_constructor(&mut class.constant_pool)?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC,
        "<init>",
        "(J)V",
        Some(body),
    )?;
    let body = state_run(&mut class.constant_pool)?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC,
        "run",
        "()V",
        Some(body),
    )?;
    Ok(class)
}

fn native_cleaner_class() -> Result<ClassFile<'static>> {
    let mut class = new_class(
        CLEANER_CLASS,
        "java/lang/Object",
        ClassAccessFlags::PUBLIC | ClassAccessFlags::FINAL | ClassAccessFlags::SUPER,
        &[],
    )?;
    add_field(
        &mut class,
        FieldAccessFlags::PRIVATE | FieldAccessFlags::STATIC | FieldAccessFlags::FINAL,
        "CLEANER",
        "Ljava/lang/ref/Cleaner;",
    )?;
    let body = cleaner_clinit(&mut class.constant_pool)?;
    add_method(
        &mut class,
        MethodAccessFlags::STATIC,
        "<clinit>",
        "()V",
        Some(body),
    )?;
    let constructor = object_constructor(&mut class.constant_pool)?;
    add_method(
        &mut class,
        MethodAccessFlags::PRIVATE,
        "<init>",
        "()V",
        Some(constructor),
    )?;
    let body = cleaner_register(&mut class.constant_pool)?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
        "register",
        "(Ljava/lang/Object;J)Ljava/lang/ref/Cleaner$Cleanable;",
        Some(body),
    )?;
    Ok(class)
}

fn native_probe_class() -> Result<ClassFile<'static>> {
    let mut class = new_class(
        PROBE_CLASS,
        "java/lang/Object",
        ClassAccessFlags::PUBLIC | ClassAccessFlags::FINAL | ClassAccessFlags::SUPER,
        &["java/lang/AutoCloseable"],
    )?;
    add_field(
        &mut class,
        FieldAccessFlags::PRIVATE | FieldAccessFlags::FINAL,
        "handle",
        "J",
    )?;
    add_field(
        &mut class,
        FieldAccessFlags::PRIVATE | FieldAccessFlags::FINAL,
        "cleanable",
        "Ljava/lang/ref/Cleaner$Cleanable;",
    )?;
    let body = probe_constructor(&mut class.constant_pool)?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC,
        "<init>",
        "()V",
        Some(body),
    )?;
    let body = clean_method(&mut class.constant_pool, PROBE_CLASS, "cleanable")?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC,
        "close",
        "()V",
        Some(body),
    )?;
    let body = getter_long(&mut class.constant_pool, PROBE_CLASS, "handle")?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC,
        "nativeHandle",
        "()J",
        Some(body),
    )?;
    Ok(class)
}

fn native_invocation_handler_class() -> Result<ClassFile<'static>> {
    let mut class = new_class(
        HANDLER_CLASS,
        "java/lang/Object",
        ClassAccessFlags::PUBLIC | ClassAccessFlags::FINAL | ClassAccessFlags::SUPER,
        &["java/lang/reflect/InvocationHandler"],
    )?;
    for (name, descriptor) in [
        ("kind", "I"),
        ("identifier", "Ljava/lang/String;"),
        ("userData", "Ljava/lang/Object;"),
        ("marker", "Ljava/lang/Object;"),
        ("track", "Ljava/lang/Object;"),
        ("listeners", "Ljava/util/ArrayList;"),
        ("paused", "Z"),
        ("volume", "I"),
        ("delivered", "Z"),
        ("position", "J"),
        ("handle", "J"),
        ("cleanable", "Ljava/lang/ref/Cleaner$Cleanable;"),
    ] {
        add_field(&mut class, FieldAccessFlags::PRIVATE, name, descriptor)?;
    }
    let body = invocation_handler_constructor(&mut class.constant_pool)?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC,
        "<init>",
        "(ILjava/lang/String;)V",
        Some(body),
    )?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC | MethodAccessFlags::NATIVE,
        "invoke",
        "(Ljava/lang/Object;Ljava/lang/reflect/Method;[Ljava/lang/Object;)Ljava/lang/Object;",
        None,
    )?;
    Ok(class)
}

fn native_load_future_class() -> Result<ClassFile<'static>> {
    let mut class = new_class(
        LOAD_FUTURE_CLASS,
        "java/util/concurrent/CompletableFuture",
        ClassAccessFlags::PUBLIC | ClassAccessFlags::FINAL | ClassAccessFlags::SUPER,
        &[],
    )?;
    add_field(
        &mut class,
        FieldAccessFlags::PRIVATE | FieldAccessFlags::FINAL,
        "handle",
        "J",
    )?;
    let constructor = load_future_constructor(&mut class.constant_pool)?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC,
        "<init>",
        "(J)V",
        Some(constructor),
    )?;
    let cancel = load_future_cancel(&mut class.constant_pool)?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC,
        "cancel",
        "(Z)Z",
        Some(cancel),
    )?;
    Ok(class)
}

fn native_load_callback_class() -> Result<ClassFile<'static>> {
    let mut class = new_class(
        LOAD_CALLBACK_CLASS,
        "java/lang/Object",
        ClassAccessFlags::PUBLIC | ClassAccessFlags::FINAL | ClassAccessFlags::SUPER,
        &["java/lang/Runnable"],
    )?;
    add_field(
        &mut class,
        FieldAccessFlags::PRIVATE | FieldAccessFlags::FINAL,
        "handle",
        "J",
    )?;
    let constructor = load_callback_constructor(&mut class.constant_pool)?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC,
        "<init>",
        "(J)V",
        Some(constructor),
    )?;
    let run = load_callback_run(&mut class.constant_pool)?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC,
        "run",
        "()V",
        Some(run),
    )?;
    Ok(class)
}

fn native_loader_class() -> Result<ClassFile<'static>> {
    let mut class = new_class(
        LOADER_CLASS,
        "java/lang/Object",
        ClassAccessFlags::PUBLIC | ClassAccessFlags::FINAL | ClassAccessFlags::SUPER,
        &[],
    )?;
    let constructor = object_constructor(&mut class.constant_pool)?;
    add_method(
        &mut class,
        MethodAccessFlags::PRIVATE,
        "<init>",
        "()V",
        Some(constructor),
    )?;
    let system = class.constant_pool.add_class("java/lang/System")?;
    let load = class
        .constant_pool
        .add_method_ref(system, "load", "(Ljava/lang/String;)V")?;
    let body = code(
        &mut class.constant_pool,
        1,
        1,
        vec![
            Instruction::Aload_0,
            Instruction::Invokestatic(load),
            Instruction::Return,
        ],
    )?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
        "load",
        "(Ljava/lang/String;)V",
        Some(body),
    )?;
    let manager = class.constant_pool.add_class(MANAGER_CLASS)?;
    let shutdown = class
        .constant_pool
        .add_method_ref(manager, "shutdown", "()V")?;
    let body = code(
        &mut class.constant_pool,
        1,
        1,
        vec![
            Instruction::Aload_0,
            Instruction::Checkcast(manager),
            Instruction::Invokevirtual(shutdown),
            Instruction::Return,
        ],
    )?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
        "shutdown",
        "(Ljava/lang/Object;)V",
        Some(body),
    )?;
    Ok(class)
}

fn native_audio_data_format_class() -> Result<ClassFile<'static>> {
    let mut class = new_class(
        FORMAT_CLASS,
        "com/sedmelluq/discord/lavaplayer/format/AudioDataFormat",
        ClassAccessFlags::PUBLIC | ClassAccessFlags::FINAL | ClassAccessFlags::SUPER,
        &[],
    )?;
    let constructor = native_audio_data_format_constructor(&mut class.constant_pool)?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC,
        "<init>",
        "(III)V",
        Some(constructor),
    )?;
    let codec = string_return(&mut class.constant_pool, "OPUS", 1)?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC,
        "codecName",
        "()Ljava/lang/String;",
        Some(codec),
    )?;
    for (name, descriptor) in [
        ("silenceBytes", "()[B"),
        (
            "createDecoder",
            "()Lcom/sedmelluq/discord/lavaplayer/format/transcoder/AudioChunkDecoder;",
        ),
        (
            "createEncoder",
            "(Lcom/sedmelluq/discord/lavaplayer/player/AudioConfiguration;)Lcom/sedmelluq/discord/lavaplayer/format/transcoder/AudioChunkEncoder;",
        ),
    ] {
        let body = null_return(
            &mut class.constant_pool,
            if name == "createEncoder" { 2 } else { 1 },
        )?;
        add_method(
            &mut class,
            MethodAccessFlags::PUBLIC,
            name,
            descriptor,
            Some(body),
        )?;
    }
    for (name, value) in [("expectedChunkSize", 100), ("maximumChunkSize", 1_275)] {
        let body = integer_return(&mut class.constant_pool, value, 1)?;
        add_method(
            &mut class,
            MethodAccessFlags::PUBLIC,
            name,
            "()I",
            Some(body),
        )?;
    }
    Ok(class)
}

fn native_audio_frame_buffer_factory_class() -> Result<ClassFile<'static>> {
    let mut class = new_class(
        FRAME_BUFFER_FACTORY_CLASS,
        "java/lang/Object",
        ClassAccessFlags::PUBLIC | ClassAccessFlags::FINAL | ClassAccessFlags::SUPER,
        &[AUDIO_FRAME_BUFFER_FACTORY_CLASS],
    )?;
    let constructor = object_constructor(&mut class.constant_pool)?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC,
        "<init>",
        "()V",
        Some(constructor),
    )?;
    let create = null_return(&mut class.constant_pool, 4)?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC,
        "create",
        "(ILcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;Ljava/util/concurrent/atomic/AtomicBoolean;)Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioFrameBuffer;",
        Some(create),
    )?;
    Ok(class)
}

fn native_event_dispatcher_class() -> Result<ClassFile<'static>> {
    let mut class = new_class(
        EVENT_DISPATCHER_CLASS,
        "java/lang/Object",
        ClassAccessFlags::PUBLIC | ClassAccessFlags::FINAL | ClassAccessFlags::SUPER,
        &[],
    )?;
    let constructor = object_constructor(&mut class.constant_pool)?;
    add_method(
        &mut class,
        MethodAccessFlags::PRIVATE,
        "<init>",
        "()V",
        Some(constructor),
    )?;
    let dispatch = event_dispatch_body(&mut class.constant_pool)?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
        "dispatch",
        "(Lcom/sedmelluq/discord/lavaplayer/player/event/AudioEventAdapter;Lcom/sedmelluq/discord/lavaplayer/player/event/AudioEvent;)V",
        Some(dispatch),
    )?;
    Ok(class)
}

fn native_audio_player_lifecycle_class() -> Result<ClassFile<'static>> {
    let mut class = new_class(
        PLAYER_LIFECYCLE_HELPER_CLASS,
        "java/lang/Object",
        ClassAccessFlags::PUBLIC | ClassAccessFlags::FINAL | ClassAccessFlags::SUPER,
        &[],
    )?;
    let constructor = object_constructor(&mut class.constant_pool)?;
    add_method(
        &mut class,
        MethodAccessFlags::PRIVATE,
        "<init>",
        "()V",
        Some(constructor),
    )?;
    let initialise = lifecycle_helper_initialise(&mut class.constant_pool)?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
        "initialise",
        "(Ljava/lang/Runnable;Ljava/util/concurrent/ScheduledExecutorService;Ljava/util/concurrent/atomic/AtomicReference;)V",
        Some(initialise),
    )?;
    let shutdown = lifecycle_helper_shutdown(&mut class.constant_pool)?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
        "shutdown",
        "(Ljava/util/concurrent/atomic/AtomicReference;)V",
        Some(shutdown),
    )?;
    let on_event = lifecycle_helper_on_event(&mut class.constant_pool)?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
        "onEvent",
        "(Ljava/util/concurrent/ConcurrentMap;Lcom/sedmelluq/discord/lavaplayer/player/event/AudioEvent;)V",
        Some(on_event),
    )?;
    let run = lifecycle_helper_run(&mut class.constant_pool)?;
    add_method(
        &mut class,
        MethodAccessFlags::PUBLIC | MethodAccessFlags::STATIC,
        "run",
        "(Ljava/util/concurrent/ConcurrentMap;Ljava/util/concurrent/atomic/AtomicLong;)V",
        Some(run),
    )?;
    Ok(class)
}

fn new_class(
    name: &str,
    superclass: &str,
    access_flags: ClassAccessFlags,
    interfaces: &[&str],
) -> Result<ClassFile<'static>> {
    let mut pool = ConstantPool::new();
    let this_class = pool.add_class(name)?;
    let superclass_index = pool.add_class(superclass)?;
    let interfaces = interfaces
        .iter()
        .map(|interface| pool.add_class(*interface))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(ClassFile {
        // Internal glue has no public compatibility obligation. Version 49 avoids
        // StackMapTable synthesis while remaining loadable on the JDK 11 baseline.
        version: JAVA_5,
        constant_pool: pool,
        access_flags,
        this_class,
        super_class: superclass_index,
        interfaces,
        fields: Vec::new(),
        methods: Vec::new(),
        attributes: Vec::new(),
        code_source_url: None,
    })
}

fn add_field(
    class: &mut ClassFile<'static>,
    access_flags: FieldAccessFlags,
    name: &str,
    descriptor: &str,
) -> Result<()> {
    class.fields.push(Field {
        access_flags,
        name_index: class.constant_pool.add_utf8(name)?,
        descriptor_index: class.constant_pool.add_utf8(descriptor)?,
        field_type: FieldType::parse(descriptor)?,
        attributes: Vec::new(),
    });
    Ok(())
}

fn add_method(
    class: &mut ClassFile<'static>,
    access_flags: MethodAccessFlags,
    name: &str,
    descriptor: &str,
    code: Option<Attribute>,
) -> Result<()> {
    class.methods.push(Method {
        access_flags,
        name_index: class.constant_pool.add_utf8(name)?,
        descriptor_index: class.constant_pool.add_utf8(descriptor)?,
        attributes: code.into_iter().collect(),
    });
    Ok(())
}

fn code(
    pool: &mut ConstantPool<'static>,
    max_stack: u16,
    max_locals: u16,
    instructions: Vec<Instruction>,
) -> Result<Attribute> {
    code_with_exceptions(pool, max_stack, max_locals, instructions, Vec::new())
}

fn code_with_exceptions(
    pool: &mut ConstantPool<'static>,
    max_stack: u16,
    max_locals: u16,
    instructions: Vec<Instruction>,
    exception_table: Vec<ExceptionTableEntry>,
) -> Result<Attribute> {
    Ok(Attribute::Code {
        name_index: pool.add_utf8("Code")?,
        max_stack,
        max_locals,
        code: instructions,
        exception_table,
        attributes: Vec::new(),
    })
}

fn delegate_to_timed_provide(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let provider =
        pool.add_class("com/sedmelluq/discord/lavaplayer/track/playback/AudioFrameProvider")?;
    let provide = pool.add_interface_method_ref(
        provider,
        "provide",
        "(JLjava/util/concurrent/TimeUnit;)Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioFrame;",
    )?;
    let time_unit = pool.add_class("java/util/concurrent/TimeUnit")?;
    let milliseconds =
        pool.add_field_ref(time_unit, "MILLISECONDS", "Ljava/util/concurrent/TimeUnit;")?;
    let runtime = pool.add_class("java/lang/RuntimeException")?;
    let runtime_init = pool.add_method_ref(runtime, "<init>", "(Ljava/lang/Throwable;)V")?;
    let thread = pool.add_class("java/lang/Thread")?;
    let current_thread = pool.add_method_ref(thread, "currentThread", "()Ljava/lang/Thread;")?;
    let interrupt = pool.add_method_ref(thread, "interrupt", "()V")?;
    let timeout = pool.add_class("java/util/concurrent/TimeoutException")?;
    let interrupted = pool.add_class("java/lang/InterruptedException")?;
    let instructions = vec![
        Instruction::Aload_0,
        Instruction::Lconst_0,
        Instruction::Getstatic(milliseconds),
        Instruction::Invokeinterface(provide, 4),
        Instruction::Areturn,
        Instruction::Astore_1,
        Instruction::New(runtime),
        Instruction::Dup,
        Instruction::Aload_1,
        Instruction::Invokespecial(runtime_init),
        Instruction::Athrow,
        Instruction::Astore_1,
        Instruction::Invokestatic(current_thread),
        Instruction::Invokevirtual(interrupt),
        Instruction::New(runtime),
        Instruction::Dup,
        Instruction::Aload_1,
        Instruction::Invokespecial(runtime_init),
        Instruction::Athrow,
    ];
    let mut body = code_with_exceptions(
        pool,
        4,
        2,
        instructions,
        vec![
            ExceptionTableEntry {
                range_pc: 0..4,
                handler_pc: 5,
                catch_type: timeout,
            },
            ExceptionTableEntry {
                range_pc: 0..4,
                handler_pc: 11,
                catch_type: interrupted,
            },
        ],
    )?;
    let stack_map_name = pool.add_utf8("StackMapTable")?;
    let Attribute::Code { attributes, .. } = &mut body else {
        return Err("expected generated code attribute".into());
    };
    attributes.push(Attribute::StackMapTable {
        name_index: stack_map_name,
        frames: vec![
            StackFrame::SameLocals1StackItemFrame {
                frame_type: 69,
                stack: vec![VerificationType::Object {
                    cpool_index: timeout,
                }],
            },
            StackFrame::SameLocals1StackItemFrame {
                frame_type: 69,
                stack: vec![VerificationType::Object {
                    cpool_index: interrupted,
                }],
            },
        ],
    });
    Ok(body)
}

fn null_return(pool: &mut ConstantPool<'static>, max_locals: u16) -> Result<Attribute> {
    code(
        pool,
        1,
        max_locals,
        vec![Instruction::Aconst_null, Instruction::Areturn],
    )
}

fn string_return(
    pool: &mut ConstantPool<'static>,
    value: &str,
    max_locals: u16,
) -> Result<Attribute> {
    let value = pool.add_string(value)?;
    code(
        pool,
        1,
        max_locals,
        vec![Instruction::Ldc_w(value), Instruction::Areturn],
    )
}

fn integer_return(
    pool: &mut ConstantPool<'static>,
    value: i16,
    max_locals: u16,
) -> Result<Attribute> {
    let instruction = match value {
        -1 => Instruction::Iconst_m1,
        0 => Instruction::Iconst_0,
        1 => Instruction::Iconst_1,
        2 => Instruction::Iconst_2,
        3 => Instruction::Iconst_3,
        4 => Instruction::Iconst_4,
        5 => Instruction::Iconst_5,
        value if i8::try_from(value).is_ok() => Instruction::Bipush(i8::try_from(value)?),
        value => Instruction::Sipush(value),
    };
    code(pool, 1, max_locals, vec![instruction, Instruction::Ireturn])
}

fn boolean_return(
    pool: &mut ConstantPool<'static>,
    value: bool,
    max_locals: u16,
) -> Result<Attribute> {
    integer_return(pool, i16::from(value), max_locals)
}

fn void_return(pool: &mut ConstantPool<'static>, max_locals: u16) -> Result<Attribute> {
    code(pool, 0, max_locals, vec![Instruction::Return])
}

fn object_getter(
    pool: &mut ConstantPool<'static>,
    owner: &str,
    field: &str,
    descriptor: &str,
) -> Result<Attribute> {
    let owner = pool.add_class(owner)?;
    let field = pool.add_field_ref(owner, field, descriptor)?;
    code(
        pool,
        1,
        1,
        vec![
            Instruction::Aload_0,
            Instruction::Getfield(field),
            Instruction::Areturn,
        ],
    )
}

fn int_getter(pool: &mut ConstantPool<'static>, owner: &str, field: &str) -> Result<Attribute> {
    primitive_getter(pool, owner, field, "I")
}

fn long_getter(pool: &mut ConstantPool<'static>, owner: &str, field: &str) -> Result<Attribute> {
    let owner = pool.add_class(owner)?;
    let field = pool.add_field_ref(owner, field, "J")?;
    code(
        pool,
        2,
        1,
        vec![
            Instruction::Aload_0,
            Instruction::Getfield(field),
            Instruction::Lreturn,
        ],
    )
}

fn bool_getter(pool: &mut ConstantPool<'static>, owner: &str, field: &str) -> Result<Attribute> {
    primitive_getter(pool, owner, field, "Z")
}

fn int_setter(pool: &mut ConstantPool<'static>, owner: &str, field: &str) -> Result<Attribute> {
    primitive_setter(pool, owner, field, "I", Instruction::Iload_1, 2)
}

fn bool_setter(pool: &mut ConstantPool<'static>, owner: &str, field: &str) -> Result<Attribute> {
    primitive_setter(pool, owner, field, "Z", Instruction::Iload_1, 2)
}

fn long_setter(pool: &mut ConstantPool<'static>, owner: &str, field: &str) -> Result<Attribute> {
    primitive_setter(pool, owner, field, "J", Instruction::Lload_1, 3)
}

fn primitive_setter(
    pool: &mut ConstantPool<'static>,
    owner: &str,
    field: &str,
    descriptor: &str,
    load: Instruction,
    max_locals: u16,
) -> Result<Attribute> {
    let owner = pool.add_class(owner)?;
    let field = pool.add_field_ref(owner, field, descriptor)?;
    code(
        pool,
        3,
        max_locals,
        vec![
            Instruction::Aload_0,
            load,
            Instruction::Putfield(field),
            Instruction::Return,
        ],
    )
}

fn object_setter(
    pool: &mut ConstantPool<'static>,
    owner: &str,
    field: &str,
    descriptor: &str,
) -> Result<Attribute> {
    let owner = pool.add_class(owner)?;
    let field = pool.add_field_ref(owner, field, descriptor)?;
    code(
        pool,
        2,
        2,
        vec![
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Putfield(field),
            Instruction::Return,
        ],
    )
}

fn primitive_getter(
    pool: &mut ConstantPool<'static>,
    owner: &str,
    field: &str,
    descriptor: &str,
) -> Result<Attribute> {
    let owner = pool.add_class(owner)?;
    let field = pool.add_field_ref(owner, field, descriptor)?;
    code(
        pool,
        1,
        1,
        vec![
            Instruction::Aload_0,
            Instruction::Getfield(field),
            Instruction::Ireturn,
        ],
    )
}

fn object_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let object = pool.add_class("java/lang/Object")?;
    let init = pool.add_method_ref(object, "<init>", "()V")?;
    code(
        pool,
        1,
        1,
        vec![
            Instruction::Aload_0,
            Instruction::Invokespecial(init),
            Instruction::Return,
        ],
    )
}

fn functional_result_handler_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let object = pool.add_class("java/lang/Object")?;
    let object_init = pool.add_method_ref(object, "<init>", "()V")?;
    let owner = pool.add_class(FUNCTIONAL_RESULT_HANDLER_CLASS)?;
    let track_consumer =
        pool.add_field_ref(owner, "trackConsumer", "Ljava/util/function/Consumer;")?;
    let playlist_consumer =
        pool.add_field_ref(owner, "playlistConsumer", "Ljava/util/function/Consumer;")?;
    let empty_result_handler =
        pool.add_field_ref(owner, "emptyResultHandler", "Ljava/lang/Runnable;")?;
    let exception_consumer =
        pool.add_field_ref(owner, "exceptionConsumer", "Ljava/util/function/Consumer;")?;
    code(
        pool,
        2,
        5,
        vec![
            Instruction::Aload_0,
            Instruction::Invokespecial(object_init),
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Putfield(track_consumer),
            Instruction::Aload_0,
            Instruction::Aload_2,
            Instruction::Putfield(playlist_consumer),
            Instruction::Aload_0,
            Instruction::Aload_3,
            Instruction::Putfield(empty_result_handler),
            Instruction::Aload_0,
            Instruction::Aload(4),
            Instruction::Putfield(exception_consumer),
            Instruction::Return,
        ],
    )
}

fn functional_result_handler_consumer(
    pool: &mut ConstantPool<'static>,
    field_name: &str,
    field_descriptor: &str,
) -> Result<Attribute> {
    let owner = pool.add_class(FUNCTIONAL_RESULT_HANDLER_CLASS)?;
    let field = pool.add_field_ref(owner, field_name, field_descriptor)?;
    let consumer = pool.add_class("java/util/function/Consumer")?;
    let accept = pool.add_interface_method_ref(consumer, "accept", "(Ljava/lang/Object;)V")?;
    let mut body = code(
        pool,
        2,
        2,
        vec![
            Instruction::Aload_0,
            Instruction::Getfield(field),
            Instruction::Ifnull(7),
            Instruction::Aload_0,
            Instruction::Getfield(field),
            Instruction::Aload_1,
            Instruction::Invokeinterface(accept, 2),
            Instruction::Return,
        ],
    )?;
    add_same_frame(pool, &mut body, 7)?;
    Ok(body)
}

fn functional_result_handler_runnable(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class(FUNCTIONAL_RESULT_HANDLER_CLASS)?;
    let field = pool.add_field_ref(owner, "emptyResultHandler", "Ljava/lang/Runnable;")?;
    let runnable = pool.add_class("java/lang/Runnable")?;
    let run = pool.add_interface_method_ref(runnable, "run", "()V")?;
    let mut body = code(
        pool,
        1,
        1,
        vec![
            Instruction::Aload_0,
            Instruction::Getfield(field),
            Instruction::Ifnull(6),
            Instruction::Aload_0,
            Instruction::Getfield(field),
            Instruction::Invokeinterface(run, 1),
            Instruction::Return,
        ],
    )?;
    add_same_frame(pool, &mut body, 6)?;
    Ok(body)
}

fn add_same_frame(
    pool: &mut ConstantPool<'static>,
    body: &mut Attribute,
    frame_type: u8,
) -> Result<()> {
    let Attribute::Code { attributes, .. } = body else {
        return Err("expected generated code attribute".into());
    };
    attributes.push(Attribute::StackMapTable {
        name_index: pool.add_utf8("StackMapTable")?,
        frames: vec![StackFrame::SameFrame { frame_type }],
    });
    Ok(())
}

fn audio_player_lifecycle_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let object = pool.add_class("java/lang/Object")?;
    let object_init = pool.add_method_ref(object, "<init>", "()V")?;
    let owner = pool.add_class(PLAYER_LIFECYCLE_MANAGER_CLASS)?;
    let active_players = pool.add_field_ref(
        owner,
        "activePlayers",
        "Ljava/util/concurrent/ConcurrentMap;",
    )?;
    let scheduler = pool.add_field_ref(
        owner,
        "scheduler",
        "Ljava/util/concurrent/ScheduledExecutorService;",
    )?;
    let cleanup_threshold = pool.add_field_ref(
        owner,
        "cleanupThreshold",
        "Ljava/util/concurrent/atomic/AtomicLong;",
    )?;
    let scheduled_task = pool.add_field_ref(
        owner,
        "scheduledTask",
        "Ljava/util/concurrent/atomic/AtomicReference;",
    )?;
    let concurrent_hash_map = pool.add_class("java/util/concurrent/ConcurrentHashMap")?;
    let concurrent_hash_map_init = pool.add_method_ref(concurrent_hash_map, "<init>", "()V")?;
    let atomic_reference = pool.add_class("java/util/concurrent/atomic/AtomicReference")?;
    let atomic_reference_init = pool.add_method_ref(atomic_reference, "<init>", "()V")?;
    code(
        pool,
        3,
        3,
        vec![
            Instruction::Aload_0,
            Instruction::Invokespecial(object_init),
            Instruction::Aload_0,
            Instruction::New(concurrent_hash_map),
            Instruction::Dup,
            Instruction::Invokespecial(concurrent_hash_map_init),
            Instruction::Putfield(active_players),
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Putfield(scheduler),
            Instruction::Aload_0,
            Instruction::Aload_2,
            Instruction::Putfield(cleanup_threshold),
            Instruction::Aload_0,
            Instruction::New(atomic_reference),
            Instruction::Dup,
            Instruction::Invokespecial(atomic_reference_init),
            Instruction::Putfield(scheduled_task),
            Instruction::Return,
        ],
    )
}

fn audio_player_lifecycle_initialise(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class(PLAYER_LIFECYCLE_MANAGER_CLASS)?;
    let scheduler = pool.add_field_ref(
        owner,
        "scheduler",
        "Ljava/util/concurrent/ScheduledExecutorService;",
    )?;
    let scheduled_task = pool.add_field_ref(
        owner,
        "scheduledTask",
        "Ljava/util/concurrent/atomic/AtomicReference;",
    )?;
    let helper = pool.add_class(PLAYER_LIFECYCLE_HELPER_CLASS)?;
    let initialise = pool.add_method_ref(
        helper,
        "initialise",
        "(Ljava/lang/Runnable;Ljava/util/concurrent/ScheduledExecutorService;Ljava/util/concurrent/atomic/AtomicReference;)V",
    )?;
    code(
        pool,
        3,
        1,
        vec![
            Instruction::Aload_0,
            Instruction::Aload_0,
            Instruction::Getfield(scheduler),
            Instruction::Aload_0,
            Instruction::Getfield(scheduled_task),
            Instruction::Invokestatic(initialise),
            Instruction::Return,
        ],
    )
}

fn audio_player_lifecycle_shutdown(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class(PLAYER_LIFECYCLE_MANAGER_CLASS)?;
    let scheduled_task = pool.add_field_ref(
        owner,
        "scheduledTask",
        "Ljava/util/concurrent/atomic/AtomicReference;",
    )?;
    let helper = pool.add_class(PLAYER_LIFECYCLE_HELPER_CLASS)?;
    let shutdown = pool.add_method_ref(
        helper,
        "shutdown",
        "(Ljava/util/concurrent/atomic/AtomicReference;)V",
    )?;
    code(
        pool,
        1,
        1,
        vec![
            Instruction::Aload_0,
            Instruction::Getfield(scheduled_task),
            Instruction::Invokestatic(shutdown),
            Instruction::Return,
        ],
    )
}

fn audio_player_lifecycle_on_event(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class(PLAYER_LIFECYCLE_MANAGER_CLASS)?;
    let active_players = pool.add_field_ref(
        owner,
        "activePlayers",
        "Ljava/util/concurrent/ConcurrentMap;",
    )?;
    let helper = pool.add_class(PLAYER_LIFECYCLE_HELPER_CLASS)?;
    let on_event = pool.add_method_ref(
        helper,
        "onEvent",
        "(Ljava/util/concurrent/ConcurrentMap;Lcom/sedmelluq/discord/lavaplayer/player/event/AudioEvent;)V",
    )?;
    code(
        pool,
        2,
        2,
        vec![
            Instruction::Aload_0,
            Instruction::Getfield(active_players),
            Instruction::Aload_1,
            Instruction::Invokestatic(on_event),
            Instruction::Return,
        ],
    )
}

fn audio_player_lifecycle_run(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class(PLAYER_LIFECYCLE_MANAGER_CLASS)?;
    let active_players = pool.add_field_ref(
        owner,
        "activePlayers",
        "Ljava/util/concurrent/ConcurrentMap;",
    )?;
    let cleanup_threshold = pool.add_field_ref(
        owner,
        "cleanupThreshold",
        "Ljava/util/concurrent/atomic/AtomicLong;",
    )?;
    let helper = pool.add_class(PLAYER_LIFECYCLE_HELPER_CLASS)?;
    let run = pool.add_method_ref(
        helper,
        "run",
        "(Ljava/util/concurrent/ConcurrentMap;Ljava/util/concurrent/atomic/AtomicLong;)V",
    )?;
    code(
        pool,
        2,
        1,
        vec![
            Instruction::Aload_0,
            Instruction::Getfield(active_players),
            Instruction::Aload_0,
            Instruction::Getfield(cleanup_threshold),
            Instruction::Invokestatic(run),
            Instruction::Return,
        ],
    )
}

fn lifecycle_helper_initialise(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let scheduler = pool.add_class("java/util/concurrent/ScheduledExecutorService")?;
    let schedule = pool.add_interface_method_ref(
        scheduler,
        "scheduleAtFixedRate",
        "(Ljava/lang/Runnable;JJLjava/util/concurrent/TimeUnit;)Ljava/util/concurrent/ScheduledFuture;",
    )?;
    let time_unit = pool.add_class("java/util/concurrent/TimeUnit")?;
    let milliseconds =
        pool.add_field_ref(time_unit, "MILLISECONDS", "Ljava/util/concurrent/TimeUnit;")?;
    let interval = pool.add_long(10_000)?;
    let atomic_reference = pool.add_class("java/util/concurrent/atomic/AtomicReference")?;
    let compare_and_set = pool.add_method_ref(
        atomic_reference,
        "compareAndSet",
        "(Ljava/lang/Object;Ljava/lang/Object;)Z",
    )?;
    let scheduled_future = pool.add_class("java/util/concurrent/ScheduledFuture")?;
    let cancel = pool.add_interface_method_ref(scheduled_future, "cancel", "(Z)Z")?;
    code(
        pool,
        7,
        4,
        vec![
            Instruction::Aload_1,
            Instruction::Aload_0,
            Instruction::Ldc2_w(interval),
            Instruction::Ldc2_w(interval),
            Instruction::Getstatic(milliseconds),
            Instruction::Invokeinterface(schedule, 7),
            Instruction::Astore_3,
            Instruction::Aload_2,
            Instruction::Aconst_null,
            Instruction::Aload_3,
            Instruction::Invokevirtual(compare_and_set),
            Instruction::Ifne(16),
            Instruction::Aload_3,
            Instruction::Iconst_0,
            Instruction::Invokeinterface(cancel, 2),
            Instruction::Pop,
            Instruction::Return,
        ],
    )
}

fn lifecycle_helper_shutdown(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let atomic_reference = pool.add_class("java/util/concurrent/atomic/AtomicReference")?;
    let get_and_set = pool.add_method_ref(
        atomic_reference,
        "getAndSet",
        "(Ljava/lang/Object;)Ljava/lang/Object;",
    )?;
    let scheduled_future = pool.add_class("java/util/concurrent/ScheduledFuture")?;
    let cancel = pool.add_interface_method_ref(scheduled_future, "cancel", "(Z)Z")?;
    code(
        pool,
        2,
        2,
        vec![
            Instruction::Aload_0,
            Instruction::Aconst_null,
            Instruction::Invokevirtual(get_and_set),
            Instruction::Checkcast(scheduled_future),
            Instruction::Astore_1,
            Instruction::Aload_1,
            Instruction::Ifnull(11),
            Instruction::Aload_1,
            Instruction::Iconst_0,
            Instruction::Invokeinterface(cancel, 2),
            Instruction::Pop,
            Instruction::Return,
        ],
    )
}

fn lifecycle_helper_on_event(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let start = pool.add_class("com/sedmelluq/discord/lavaplayer/player/event/TrackStartEvent")?;
    let end = pool.add_class("com/sedmelluq/discord/lavaplayer/player/event/TrackEndEvent")?;
    let event = pool.add_class("com/sedmelluq/discord/lavaplayer/player/event/AudioEvent")?;
    let player = pool.add_field_ref(
        event,
        "player",
        "Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;",
    )?;
    let map = pool.add_class("java/util/concurrent/ConcurrentMap")?;
    let put = pool.add_interface_method_ref(
        map,
        "put",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
    )?;
    let remove =
        pool.add_interface_method_ref(map, "remove", "(Ljava/lang/Object;)Ljava/lang/Object;")?;
    code(
        pool,
        3,
        2,
        vec![
            Instruction::Aload_1,
            Instruction::Instanceof(start),
            Instruction::Ifeq(11),
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Getfield(player),
            Instruction::Aload_1,
            Instruction::Getfield(player),
            Instruction::Invokeinterface(put, 3),
            Instruction::Pop,
            Instruction::Goto(19),
            Instruction::Aload_1,
            Instruction::Instanceof(end),
            Instruction::Ifeq(19),
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Getfield(player),
            Instruction::Invokeinterface(remove, 2),
            Instruction::Pop,
            Instruction::Return,
        ],
    )
}

fn lifecycle_helper_run(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let map = pool.add_class("java/util/concurrent/ConcurrentMap")?;
    let key_set = pool.add_interface_method_ref(map, "keySet", "()Ljava/util/Set;")?;
    let set = pool.add_class("java/util/Set")?;
    let iterator = pool.add_interface_method_ref(set, "iterator", "()Ljava/util/Iterator;")?;
    let iterator_class = pool.add_class("java/util/Iterator")?;
    let has_next = pool.add_interface_method_ref(iterator_class, "hasNext", "()Z")?;
    let next = pool.add_interface_method_ref(iterator_class, "next", "()Ljava/lang/Object;")?;
    let player = pool.add_class("com/sedmelluq/discord/lavaplayer/player/AudioPlayer")?;
    let check_cleanup = pool.add_interface_method_ref(player, "checkCleanup", "(J)V")?;
    let atomic_long = pool.add_class("java/util/concurrent/atomic/AtomicLong")?;
    let get = pool.add_method_ref(atomic_long, "get", "()J")?;
    code(
        pool,
        3,
        4,
        vec![
            Instruction::Aload_0,
            Instruction::Invokeinterface(key_set, 1),
            Instruction::Invokeinterface(iterator, 1),
            Instruction::Astore_2,
            Instruction::Aload_2,
            Instruction::Invokeinterface(has_next, 1),
            Instruction::Ifeq(16),
            Instruction::Aload_2,
            Instruction::Invokeinterface(next, 1),
            Instruction::Checkcast(player),
            Instruction::Astore_3,
            Instruction::Aload_3,
            Instruction::Aload_1,
            Instruction::Invokevirtual(get),
            Instruction::Invokeinterface(check_cleanup, 3),
            Instruction::Goto(4),
            Instruction::Return,
        ],
    )
}

fn audio_processing_context_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let object = pool.add_class("java/lang/Object")?;
    let object_init = pool.add_method_ref(object, "<init>", "()V")?;
    let owner = pool.add_class(AUDIO_PROCESSING_CONTEXT_CLASS)?;
    let configuration = pool.add_field_ref(
        owner,
        "configuration",
        "Lcom/sedmelluq/discord/lavaplayer/player/AudioConfiguration;",
    )?;
    let frame_buffer = pool.add_field_ref(
        owner,
        "frameBuffer",
        "Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioFrameBuffer;",
    )?;
    let player_options = pool.add_field_ref(
        owner,
        "playerOptions",
        "Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayerOptions;",
    )?;
    let output_format = pool.add_field_ref(
        owner,
        "outputFormat",
        "Lcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;",
    )?;
    let filter_hot_swap_enabled = pool.add_field_ref(owner, "filterHotSwapEnabled", "Z")?;
    let configuration_class = pool.add_class(CONFIGURATION_CLASS)?;
    let is_filter_hot_swap_enabled =
        pool.add_method_ref(configuration_class, "isFilterHotSwapEnabled", "()Z")?;
    code(
        pool,
        2,
        5,
        vec![
            Instruction::Aload_0,
            Instruction::Invokespecial(object_init),
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Putfield(configuration),
            Instruction::Aload_0,
            Instruction::Aload_2,
            Instruction::Putfield(frame_buffer),
            Instruction::Aload_0,
            Instruction::Aload_3,
            Instruction::Putfield(player_options),
            Instruction::Aload_0,
            Instruction::Aload(4),
            Instruction::Putfield(output_format),
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Invokevirtual(is_filter_hot_swap_enabled),
            Instruction::Putfield(filter_hot_swap_enabled),
            Instruction::Return,
        ],
    )
}

fn decoded_track_holder_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let object = pool.add_class("java/lang/Object")?;
    let object_init = pool.add_method_ref(object, "<init>", "()V")?;
    let owner = pool.add_class(DECODED_TRACK_HOLDER_CLASS)?;
    let decoded_track = pool.add_field_ref(
        owner,
        "decodedTrack",
        "Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;",
    )?;
    code(
        pool,
        2,
        2,
        vec![
            Instruction::Aload_0,
            Instruction::Invokespecial(object_init),
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Putfield(decoded_track),
            Instruction::Return,
        ],
    )
}

fn audio_player_options_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let object = pool.add_class("java/lang/Object")?;
    let object_init = pool.add_method_ref(object, "<init>", "()V")?;
    let owner = pool.add_class(AUDIO_PLAYER_OPTIONS_CLASS)?;
    let atomic_integer = pool.add_class("java/util/concurrent/atomic/AtomicInteger")?;
    let atomic_integer_init = pool.add_method_ref(atomic_integer, "<init>", "(I)V")?;
    let atomic_reference = pool.add_class("java/util/concurrent/atomic/AtomicReference")?;
    let atomic_reference_init = pool.add_method_ref(atomic_reference, "<init>", "()V")?;
    let volume_level = pool.add_field_ref(
        owner,
        "volumeLevel",
        "Ljava/util/concurrent/atomic/AtomicInteger;",
    )?;
    let filter_factory = pool.add_field_ref(
        owner,
        "filterFactory",
        "Ljava/util/concurrent/atomic/AtomicReference;",
    )?;
    let frame_buffer_duration = pool.add_field_ref(
        owner,
        "frameBufferDuration",
        "Ljava/util/concurrent/atomic/AtomicReference;",
    )?;
    code(
        pool,
        4,
        1,
        vec![
            Instruction::Aload_0,
            Instruction::Invokespecial(object_init),
            Instruction::Aload_0,
            Instruction::New(atomic_integer),
            Instruction::Dup,
            Instruction::Bipush(100),
            Instruction::Invokespecial(atomic_integer_init),
            Instruction::Putfield(volume_level),
            Instruction::Aload_0,
            Instruction::New(atomic_reference),
            Instruction::Dup,
            Instruction::Invokespecial(atomic_reference_init),
            Instruction::Putfield(filter_factory),
            Instruction::Aload_0,
            Instruction::New(atomic_reference),
            Instruction::Dup,
            Instruction::Invokespecial(atomic_reference_init),
            Instruction::Putfield(frame_buffer_duration),
            Instruction::Return,
        ],
    )
}

fn unsupported_body(
    pool: &mut ConstantPool<'static>,
    message: &str,
    max_locals: u16,
) -> Result<Attribute> {
    let exception = pool.add_class("java/lang/UnsupportedOperationException")?;
    let init = pool.add_method_ref(exception, "<init>", "(Ljava/lang/String;)V")?;
    let message = pool.add_string(message)?;
    code(
        pool,
        3,
        max_locals,
        vec![
            Instruction::New(exception),
            Instruction::Dup,
            Instruction::Ldc_w(message),
            Instruction::Invokespecial(init),
            Instruction::Athrow,
        ],
    )
}

fn unsupported_without_message(
    pool: &mut ConstantPool<'static>,
    max_locals: u16,
) -> Result<Attribute> {
    let exception = pool.add_class("java/lang/UnsupportedOperationException")?;
    let init = pool.add_method_ref(exception, "<init>", "()V")?;
    code(
        pool,
        2,
        max_locals,
        vec![
            Instruction::New(exception),
            Instruction::Dup,
            Instruction::Invokespecial(init),
            Instruction::Athrow,
        ],
    )
}

fn mutable_frame_freeze(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class(ABSTRACT_MUTABLE_FRAME_CLASS)?;
    let immutable = pool.add_class(IMMUTABLE_FRAME_CLASS)?;
    let timecode = pool.add_field_ref(owner, "timecode", "J")?;
    let volume = pool.add_field_ref(owner, "volume", "I")?;
    let format = pool.add_field_ref(
        owner,
        "format",
        "Lcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;",
    )?;
    let get_data = pool.add_method_ref(owner, "getData", "()[B")?;
    let init = pool.add_method_ref(
        immutable,
        "<init>",
        "(J[BILcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;)V",
    )?;
    code(
        pool,
        7,
        1,
        vec![
            Instruction::New(immutable),
            Instruction::Dup,
            Instruction::Aload_0,
            Instruction::Getfield(timecode),
            Instruction::Aload_0,
            Instruction::Invokevirtual(get_data),
            Instruction::Aload_0,
            Instruction::Getfield(volume),
            Instruction::Aload_0,
            Instruction::Getfield(format),
            Instruction::Invokespecial(init),
            Instruction::Areturn,
        ],
    )
}

fn immutable_frame_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let object = pool.add_class("java/lang/Object")?;
    let object_init = pool.add_method_ref(object, "<init>", "()V")?;
    let owner = pool.add_class(IMMUTABLE_FRAME_CLASS)?;
    let timecode = pool.add_field_ref(owner, "timecode", "J")?;
    let data = pool.add_field_ref(owner, "data", "[B")?;
    let volume = pool.add_field_ref(owner, "volume", "I")?;
    let format = pool.add_field_ref(
        owner,
        "format",
        "Lcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;",
    )?;
    code(
        pool,
        3,
        6,
        vec![
            Instruction::Aload_0,
            Instruction::Invokespecial(object_init),
            Instruction::Aload_0,
            Instruction::Lload_1,
            Instruction::Putfield(timecode),
            Instruction::Aload_0,
            Instruction::Aload_3,
            Instruction::Putfield(data),
            Instruction::Aload_0,
            Instruction::Iload(4),
            Instruction::Putfield(volume),
            Instruction::Aload_0,
            Instruction::Aload(5),
            Instruction::Putfield(format),
            Instruction::Return,
        ],
    )
}

fn immutable_frame_data_length(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class(IMMUTABLE_FRAME_CLASS)?;
    let data = pool.add_field_ref(owner, "data", "[B")?;
    code(
        pool,
        1,
        1,
        vec![
            Instruction::Aload_0,
            Instruction::Getfield(data),
            Instruction::Arraylength,
            Instruction::Ireturn,
        ],
    )
}

fn immutable_frame_copy_data(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class(IMMUTABLE_FRAME_CLASS)?;
    let data = pool.add_field_ref(owner, "data", "[B")?;
    let system = pool.add_class("java/lang/System")?;
    let arraycopy = pool.add_method_ref(
        system,
        "arraycopy",
        "(Ljava/lang/Object;ILjava/lang/Object;II)V",
    )?;
    code(
        pool,
        5,
        3,
        vec![
            Instruction::Aload_0,
            Instruction::Getfield(data),
            Instruction::Iconst_0,
            Instruction::Aload_1,
            Instruction::Iload_2,
            Instruction::Aload_0,
            Instruction::Getfield(data),
            Instruction::Arraylength,
            Instruction::Invokestatic(arraycopy),
            Instruction::Return,
        ],
    )
}

fn mutable_frame_constructor(
    pool: &mut ConstantPool<'static>,
    with_buffer: bool,
) -> Result<Attribute> {
    let parent = pool.add_class(ABSTRACT_MUTABLE_FRAME_CLASS)?;
    let parent_init = pool.add_method_ref(parent, "<init>", "()V")?;
    let mut instructions = vec![
        Instruction::Aload_0,
        Instruction::Invokespecial(parent_init),
    ];
    if with_buffer {
        let owner = pool.add_class(MUTABLE_FRAME_CLASS)?;
        let set_buffer = pool.add_method_ref(owner, "setBuffer", "(Ljava/nio/ByteBuffer;)V")?;
        instructions.extend([
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Invokevirtual(set_buffer),
        ]);
    }
    instructions.push(Instruction::Return);
    code(pool, 2, if with_buffer { 2 } else { 1 }, instructions)
}

fn mutable_frame_set_buffer(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class(MUTABLE_FRAME_CLASS)?;
    let frame_buffer = pool.add_field_ref(owner, "frameBuffer", "Ljava/nio/ByteBuffer;")?;
    let frame_position = pool.add_field_ref(owner, "framePosition", "I")?;
    let frame_length = pool.add_field_ref(owner, "frameLength", "I")?;
    let byte_buffer = pool.add_class("java/nio/ByteBuffer")?;
    let position = pool.add_method_ref(byte_buffer, "position", "()I")?;
    let remaining = pool.add_method_ref(byte_buffer, "remaining", "()I")?;
    code(
        pool,
        2,
        2,
        vec![
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Putfield(frame_buffer),
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Invokevirtual(position),
            Instruction::Putfield(frame_position),
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Invokevirtual(remaining),
            Instruction::Putfield(frame_length),
            Instruction::Return,
        ],
    )
}

fn mutable_frame_store(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class(MUTABLE_FRAME_CLASS)?;
    let frame_buffer = pool.add_field_ref(owner, "frameBuffer", "Ljava/nio/ByteBuffer;")?;
    let frame_position = pool.add_field_ref(owner, "framePosition", "I")?;
    let frame_length = pool.add_field_ref(owner, "frameLength", "I")?;
    let byte_buffer = pool.add_class("java/nio/ByteBuffer")?;
    let position = pool.add_method_ref(byte_buffer, "position", "(I)Ljava/nio/ByteBuffer;")?;
    let capacity = pool.add_method_ref(byte_buffer, "capacity", "()I")?;
    let limit = pool.add_method_ref(byte_buffer, "limit", "(I)Ljava/nio/ByteBuffer;")?;
    let put = pool.add_method_ref(byte_buffer, "put", "([BII)Ljava/nio/ByteBuffer;")?;
    code(
        pool,
        4,
        4,
        vec![
            Instruction::Aload_0,
            Instruction::Getfield(frame_buffer),
            Instruction::Aload_0,
            Instruction::Getfield(frame_position),
            Instruction::Invokevirtual(position),
            Instruction::Pop,
            Instruction::Aload_0,
            Instruction::Getfield(frame_buffer),
            Instruction::Aload_0,
            Instruction::Getfield(frame_buffer),
            Instruction::Invokevirtual(capacity),
            Instruction::Invokevirtual(limit),
            Instruction::Pop,
            Instruction::Aload_0,
            Instruction::Getfield(frame_buffer),
            Instruction::Aload_1,
            Instruction::Iload_2,
            Instruction::Iload_3,
            Instruction::Invokevirtual(put),
            Instruction::Pop,
            Instruction::Aload_0,
            Instruction::Iload_3,
            Instruction::Putfield(frame_length),
            Instruction::Return,
        ],
    )
}

fn mutable_frame_get_data(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    frame_get_data_copy(pool, MUTABLE_FRAME_CLASS)
}

fn frame_get_data_copy(pool: &mut ConstantPool<'static>, class_name: &str) -> Result<Attribute> {
    let owner = pool.add_class(class_name)?;
    let get_length = pool.add_method_ref(owner, "getDataLength", "()I")?;
    let copy = pool.add_method_ref(owner, "getData", "([BI)V")?;
    code(
        pool,
        3,
        2,
        vec![
            Instruction::Aload_0,
            Instruction::Invokevirtual(get_length),
            Instruction::Newarray(ArrayType::Byte),
            Instruction::Astore_1,
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Iconst_0,
            Instruction::Invokevirtual(copy),
            Instruction::Aload_1,
            Instruction::Areturn,
        ],
    )
}

fn reference_mutable_frame_end_offset(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class(REFERENCE_MUTABLE_FRAME_CLASS)?;
    let frame_offset = pool.add_field_ref(owner, "frameOffset", "I")?;
    let frame_length = pool.add_field_ref(owner, "frameLength", "I")?;
    code(
        pool,
        2,
        1,
        vec![
            Instruction::Aload_0,
            Instruction::Getfield(frame_offset),
            Instruction::Aload_0,
            Instruction::Getfield(frame_length),
            Instruction::Iadd,
            Instruction::Ireturn,
        ],
    )
}

fn reference_mutable_frame_copy_data(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class(REFERENCE_MUTABLE_FRAME_CLASS)?;
    let frame_buffer = pool.add_field_ref(owner, "frameBuffer", "[B")?;
    let frame_offset = pool.add_field_ref(owner, "frameOffset", "I")?;
    let frame_length = pool.add_field_ref(owner, "frameLength", "I")?;
    let system = pool.add_class("java/lang/System")?;
    let arraycopy = pool.add_method_ref(
        system,
        "arraycopy",
        "(Ljava/lang/Object;ILjava/lang/Object;II)V",
    )?;
    code(
        pool,
        5,
        3,
        vec![
            Instruction::Aload_0,
            Instruction::Getfield(frame_buffer),
            Instruction::Aload_0,
            Instruction::Getfield(frame_offset),
            Instruction::Aload_1,
            Instruction::Iload_2,
            Instruction::Aload_0,
            Instruction::Getfield(frame_length),
            Instruction::Invokestatic(arraycopy),
            Instruction::Return,
        ],
    )
}

fn reference_mutable_frame_set_data(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class(REFERENCE_MUTABLE_FRAME_CLASS)?;
    let frame_buffer = pool.add_field_ref(owner, "frameBuffer", "[B")?;
    let frame_offset = pool.add_field_ref(owner, "frameOffset", "I")?;
    let frame_length = pool.add_field_ref(owner, "frameLength", "I")?;
    code(
        pool,
        2,
        4,
        vec![
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Putfield(frame_buffer),
            Instruction::Aload_0,
            Instruction::Iload_2,
            Instruction::Putfield(frame_offset),
            Instruction::Aload_0,
            Instruction::Iload_3,
            Instruction::Putfield(frame_length),
            Instruction::Return,
        ],
    )
}

fn mutable_frame_copy_data(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class(MUTABLE_FRAME_CLASS)?;
    let frame_buffer = pool.add_field_ref(owner, "frameBuffer", "Ljava/nio/ByteBuffer;")?;
    let frame_position = pool.add_field_ref(owner, "framePosition", "I")?;
    let frame_length = pool.add_field_ref(owner, "frameLength", "I")?;
    let byte_buffer = pool.add_class("java/nio/ByteBuffer")?;
    let get_position = pool.add_method_ref(byte_buffer, "position", "()I")?;
    let set_position = pool.add_method_ref(byte_buffer, "position", "(I)Ljava/nio/ByteBuffer;")?;
    let get = pool.add_method_ref(byte_buffer, "get", "([BII)Ljava/nio/ByteBuffer;")?;
    code(
        pool,
        4,
        4,
        vec![
            Instruction::Aload_0,
            Instruction::Getfield(frame_buffer),
            Instruction::Invokevirtual(get_position),
            Instruction::Istore_3,
            Instruction::Aload_0,
            Instruction::Getfield(frame_buffer),
            Instruction::Aload_0,
            Instruction::Getfield(frame_position),
            Instruction::Invokevirtual(set_position),
            Instruction::Pop,
            Instruction::Aload_0,
            Instruction::Getfield(frame_buffer),
            Instruction::Aload_1,
            Instruction::Iload_2,
            Instruction::Aload_0,
            Instruction::Getfield(frame_length),
            Instruction::Invokevirtual(get),
            Instruction::Pop,
            Instruction::Aload_0,
            Instruction::Getfield(frame_buffer),
            Instruction::Iload_3,
            Instruction::Invokevirtual(set_position),
            Instruction::Pop,
            Instruction::Return,
        ],
    )
}

fn ensure_abi_body(pool: &mut ConstantPool<'static>, expected: u8) -> Result<Attribute> {
    let native = pool.add_class(NATIVE_CLASS)?;
    let abi = pool.add_method_ref(native, "abiVersion", "()I")?;
    let exception = pool.add_class("java/lang/UnsatisfiedLinkError")?;
    let init = pool.add_method_ref(exception, "<init>", "(Ljava/lang/String;)V")?;
    let message = pool.add_string(format!(
        "Mantle compatibility JAR expects native ABI {expected}; loaded library returned another ABI"
    ))?;
    let expected_instruction = match expected {
        0 => Instruction::Iconst_0,
        1 => Instruction::Iconst_1,
        2 => Instruction::Iconst_2,
        3 => Instruction::Iconst_3,
        4 => Instruction::Iconst_4,
        5 => Instruction::Iconst_5,
        value => Instruction::Bipush(i8::try_from(value)?),
    };
    code(
        pool,
        3,
        0,
        vec![
            Instruction::Invokestatic(abi),
            expected_instruction,
            Instruction::If_icmpeq(8),
            Instruction::New(exception),
            Instruction::Dup,
            Instruction::Ldc_w(message),
            Instruction::Invokespecial(init),
            Instruction::Athrow,
            Instruction::Return,
        ],
    )
}

fn manager_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let object = pool.add_class("java/lang/Object")?;
    let object_init = pool.add_method_ref(object, "<init>", "()V")?;
    let native = pool.add_class(NATIVE_CLASS)?;
    let ensure = pool.add_method_ref(native, "ensureAbi", "()V")?;
    let create = pool.add_method_ref(native, "createHandle", "(I)J")?;
    let manager = pool.add_class(MANAGER_CLASS)?;
    let handle = pool.add_field_ref(manager, "mantleHandle", "J")?;
    let cleanable = pool.add_field_ref(
        manager,
        "mantleCleanable",
        "Ljava/lang/ref/Cleaner$Cleanable;",
    )?;
    let sources = pool.add_field_ref(manager, "mantleSources", "Ljava/util/ArrayList;")?;
    let list = pool.add_class("java/util/ArrayList")?;
    let list_init = pool.add_method_ref(list, "<init>", "()V")?;
    let cleaner = pool.add_class(CLEANER_CLASS)?;
    let register = pool.add_method_ref(
        cleaner,
        "register",
        "(Ljava/lang/Object;J)Ljava/lang/ref/Cleaner$Cleanable;",
    )?;
    code(
        pool,
        4,
        1,
        vec![
            Instruction::Aload_0,
            Instruction::Invokespecial(object_init),
            Instruction::Aload_0,
            Instruction::New(list),
            Instruction::Dup,
            Instruction::Invokespecial(list_init),
            Instruction::Putfield(sources),
            Instruction::Invokestatic(ensure),
            Instruction::Aload_0,
            Instruction::Iconst_1,
            Instruction::Invokestatic(create),
            Instruction::Putfield(handle),
            Instruction::Aload_0,
            Instruction::Aload_0,
            Instruction::Aload_0,
            Instruction::Getfield(handle),
            Instruction::Invokestatic(register),
            Instruction::Putfield(cleanable),
            Instruction::Return,
        ],
    )
}

fn manager_shutdown(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    clean_method(pool, MANAGER_CLASS, "mantleCleanable")
}

fn manager_configuration(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let configuration = pool.add_class(CONFIGURATION_CLASS)?;
    let init = pool.add_method_ref(configuration, "<init>", "()V")?;
    code(
        pool,
        2,
        1,
        vec![
            Instruction::New(configuration),
            Instruction::Dup,
            Instruction::Invokespecial(init),
            Instruction::Areturn,
        ],
    )
}

fn manager_create_player(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let native = pool.add_class(NATIVE_CLASS)?;
    let create = pool.add_method_ref(
        native,
        "createProxy",
        "(ILjava/lang/String;)Ljava/lang/Object;",
    )?;
    let player = pool.add_class("com/sedmelluq/discord/lavaplayer/player/AudioPlayer")?;
    code(
        pool,
        2,
        1,
        vec![
            Instruction::Iconst_2,
            Instruction::Aconst_null,
            Instruction::Invokestatic(create),
            Instruction::Checkcast(player),
            Instruction::Areturn,
        ],
    )
}

fn manager_load_string(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let native = pool.add_class(NATIVE_CLASS)?;
    let load = pool.add_method_ref(
        native,
        "loadItem",
        "(Ljava/lang/String;Lcom/sedmelluq/discord/lavaplayer/player/AudioLoadResultHandler;)Ljava/util/concurrent/Future;",
    )?;
    code(
        pool,
        2,
        3,
        vec![
            Instruction::Aload_1,
            Instruction::Aload_2,
            Instruction::Invokestatic(load),
            Instruction::Areturn,
        ],
    )
}

fn manager_load_reference(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let native = pool.add_class(NATIVE_CLASS)?;
    let load = pool.add_method_ref(
        native,
        "loadItemReference",
        "(Lcom/sedmelluq/discord/lavaplayer/track/AudioReference;Lcom/sedmelluq/discord/lavaplayer/player/AudioLoadResultHandler;)Ljava/util/concurrent/Future;",
    )?;
    code(
        pool,
        2,
        3,
        vec![
            Instruction::Aload_1,
            Instruction::Aload_2,
            Instruction::Invokestatic(load),
            Instruction::Areturn,
        ],
    )
}

fn manager_load_ordered_string(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let native = pool.add_class(NATIVE_CLASS)?;
    let load = pool.add_method_ref(
        native,
        "loadItemOrdered",
        "(Ljava/lang/Object;Ljava/lang/String;Lcom/sedmelluq/discord/lavaplayer/player/AudioLoadResultHandler;)Ljava/util/concurrent/Future;",
    )?;
    code(
        pool,
        3,
        4,
        vec![
            Instruction::Aload_1,
            Instruction::Aload_2,
            Instruction::Aload_3,
            Instruction::Invokestatic(load),
            Instruction::Areturn,
        ],
    )
}

fn manager_load_ordered_reference(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let native = pool.add_class(NATIVE_CLASS)?;
    let load = pool.add_method_ref(
        native,
        "loadItemOrderedReference",
        "(Ljava/lang/Object;Lcom/sedmelluq/discord/lavaplayer/track/AudioReference;Lcom/sedmelluq/discord/lavaplayer/player/AudioLoadResultHandler;)Ljava/util/concurrent/Future;",
    )?;
    code(
        pool,
        3,
        4,
        vec![
            Instruction::Aload_1,
            Instruction::Aload_2,
            Instruction::Aload_3,
            Instruction::Invokestatic(load),
            Instruction::Areturn,
        ],
    )
}

fn manager_register_source(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let manager = pool.add_class(MANAGER_CLASS)?;
    let sources = pool.add_field_ref(manager, "mantleSources", "Ljava/util/ArrayList;")?;
    let list = pool.add_class("java/util/ArrayList")?;
    let add = pool.add_method_ref(list, "add", "(Ljava/lang/Object;)Z")?;
    let native = pool.add_class(NATIVE_CLASS)?;
    let register = pool.add_method_ref(
        native,
        "registerSourceManager",
        "(Lcom/sedmelluq/discord/lavaplayer/player/DefaultAudioPlayerManager;Lcom/sedmelluq/discord/lavaplayer/source/AudioSourceManager;)V",
    )?;
    code(
        pool,
        2,
        2,
        vec![
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Invokestatic(register),
            Instruction::Aload_0,
            Instruction::Getfield(sources),
            Instruction::Aload_1,
            Instruction::Invokevirtual(add),
            Instruction::Pop,
            Instruction::Return,
        ],
    )
}

fn manager_source(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let native = pool.add_class(NATIVE_CLASS)?;
    let source = pool.add_method_ref(
        native,
        "sourceManager",
        "(Lcom/sedmelluq/discord/lavaplayer/player/DefaultAudioPlayerManager;Ljava/lang/Class;)Lcom/sedmelluq/discord/lavaplayer/source/AudioSourceManager;",
    )?;
    code(
        pool,
        2,
        2,
        vec![
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Invokestatic(source),
            Instruction::Areturn,
        ],
    )
}

fn manager_load_sync(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let native = pool.add_class(NATIVE_CLASS)?;
    let load = pool.add_method_ref(
        native,
        "loadItemSync",
        "(Lcom/sedmelluq/discord/lavaplayer/track/AudioReference;)Lcom/sedmelluq/discord/lavaplayer/track/AudioItem;",
    )?;
    code(
        pool,
        1,
        2,
        vec![
            Instruction::Aload_1,
            Instruction::Invokestatic(load),
            Instruction::Areturn,
        ],
    )
}

fn manager_load_sync_handled(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let native = pool.add_class(NATIVE_CLASS)?;
    let load = pool.add_method_ref(
        native,
        "loadItemSyncHandled",
        "(Lcom/sedmelluq/discord/lavaplayer/track/AudioReference;Lcom/sedmelluq/discord/lavaplayer/player/AudioLoadResultHandler;)V",
    )?;
    code(
        pool,
        2,
        3,
        vec![
            Instruction::Aload_1,
            Instruction::Aload_2,
            Instruction::Invokestatic(load),
            Instruction::Return,
        ],
    )
}

fn manager_get_sources(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let manager = pool.add_class(MANAGER_CLASS)?;
    let sources = pool.add_field_ref(manager, "mantleSources", "Ljava/util/ArrayList;")?;
    let collections = pool.add_class("java/util/Collections")?;
    let unmodifiable = pool.add_method_ref(
        collections,
        "unmodifiableList",
        "(Ljava/util/List;)Ljava/util/List;",
    )?;
    code(
        pool,
        1,
        1,
        vec![
            Instruction::Aload_0,
            Instruction::Getfield(sources),
            Instruction::Invokestatic(unmodifiable),
            Instruction::Areturn,
        ],
    )
}

fn audio_reference_short_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class("com/sedmelluq/discord/lavaplayer/track/AudioReference")?;
    let init = pool.add_method_ref(
        owner,
        "<init>",
        "(Ljava/lang/String;Ljava/lang/String;Lcom/sedmelluq/discord/lavaplayer/container/MediaContainerDescriptor;)V",
    )?;
    code(
        pool,
        4,
        3,
        vec![
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Aload_2,
            Instruction::Aconst_null,
            Instruction::Invokespecial(init),
            Instruction::Return,
        ],
    )
}

fn audio_reference_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let object = pool.add_class("java/lang/Object")?;
    let object_init = pool.add_method_ref(object, "<init>", "()V")?;
    let owner = pool.add_class("com/sedmelluq/discord/lavaplayer/track/AudioReference")?;
    let identifier = pool.add_field_ref(owner, "identifier", "Ljava/lang/String;")?;
    let title = pool.add_field_ref(owner, "title", "Ljava/lang/String;")?;
    let container = pool.add_field_ref(
        owner,
        "containerDescriptor",
        "Lcom/sedmelluq/discord/lavaplayer/container/MediaContainerDescriptor;",
    )?;
    code(
        pool,
        2,
        4,
        vec![
            Instruction::Aload_0,
            Instruction::Invokespecial(object_init),
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Putfield(identifier),
            Instruction::Aload_0,
            Instruction::Aload_2,
            Instruction::Putfield(title),
            Instruction::Aload_0,
            Instruction::Aload_3,
            Instruction::Putfield(container),
            Instruction::Return,
        ],
    )
}

fn basic_playlist_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let object = pool.add_class("java/lang/Object")?;
    let object_init = pool.add_method_ref(object, "<init>", "()V")?;
    let owner = pool.add_class(BASIC_PLAYLIST_CLASS)?;
    let name = pool.add_field_ref(owner, "mantleName", "Ljava/lang/String;")?;
    let tracks = pool.add_field_ref(owner, "mantleTracks", "Ljava/util/List;")?;
    let selected = pool.add_field_ref(
        owner,
        "mantleSelectedTrack",
        "Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;",
    )?;
    let search = pool.add_field_ref(owner, "mantleSearchResult", "Z")?;
    code(
        pool,
        2,
        5,
        vec![
            Instruction::Aload_0,
            Instruction::Invokespecial(object_init),
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Putfield(name),
            Instruction::Aload_0,
            Instruction::Aload_2,
            Instruction::Putfield(tracks),
            Instruction::Aload_0,
            Instruction::Aload_3,
            Instruction::Putfield(selected),
            Instruction::Aload_0,
            Instruction::Iload(4),
            Instruction::Putfield(search),
            Instruction::Return,
        ],
    )
}

fn audio_reference_clinit(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class("com/sedmelluq/discord/lavaplayer/track/AudioReference")?;
    let init = pool.add_method_ref(owner, "<init>", "(Ljava/lang/String;Ljava/lang/String;)V")?;
    let no_track = pool.add_field_ref(
        owner,
        "NO_TRACK",
        "Lcom/sedmelluq/discord/lavaplayer/track/AudioReference;",
    )?;
    code(
        pool,
        4,
        0,
        vec![
            Instruction::New(owner),
            Instruction::Dup,
            Instruction::Aconst_null,
            Instruction::Aconst_null,
            Instruction::Invokespecial(init),
            Instruction::Putstatic(no_track),
            Instruction::Return,
        ],
    )
}

fn terminator_frame_clinit(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class(TERMINATOR_FRAME_CLASS)?;
    let init = pool.add_method_ref(owner, "<init>", "()V")?;
    let instance = pool.add_field_ref(owner, "INSTANCE", &format!("L{TERMINATOR_FRAME_CLASS};"))?;
    code(
        pool,
        2,
        0,
        vec![
            Instruction::New(owner),
            Instruction::Dup,
            Instruction::Invokespecial(init),
            Instruction::Putstatic(instance),
            Instruction::Return,
        ],
    )
}

fn load_future_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let parent = pool.add_class("java/util/concurrent/CompletableFuture")?;
    let init = pool.add_method_ref(parent, "<init>", "()V")?;
    let owner = pool.add_class(LOAD_FUTURE_CLASS)?;
    let handle = pool.add_field_ref(owner, "handle", "J")?;
    code(
        pool,
        3,
        3,
        vec![
            Instruction::Aload_0,
            Instruction::Invokespecial(init),
            Instruction::Aload_0,
            Instruction::Lload_1,
            Instruction::Putfield(handle),
            Instruction::Return,
        ],
    )
}

fn load_future_cancel(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class(LOAD_FUTURE_CLASS)?;
    let handle = pool.add_field_ref(owner, "handle", "J")?;
    let native = pool.add_class(NATIVE_CLASS)?;
    let cancel_native = pool.add_method_ref(native, "cancelLoad", "(JZ)Z")?;
    let parent = pool.add_class("java/util/concurrent/CompletableFuture")?;
    let cancel_parent = pool.add_method_ref(parent, "cancel", "(Z)Z")?;
    code(
        pool,
        3,
        2,
        vec![
            Instruction::Aload_0,
            Instruction::Getfield(handle),
            Instruction::Iload_1,
            Instruction::Invokestatic(cancel_native),
            Instruction::Ifne(7),
            Instruction::Iconst_0,
            Instruction::Ireturn,
            Instruction::Aload_0,
            Instruction::Iload_1,
            Instruction::Invokespecial(cancel_parent),
            Instruction::Ireturn,
        ],
    )
}

fn load_callback_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let object = pool.add_class("java/lang/Object")?;
    let init = pool.add_method_ref(object, "<init>", "()V")?;
    let owner = pool.add_class(LOAD_CALLBACK_CLASS)?;
    let handle = pool.add_field_ref(owner, "handle", "J")?;
    code(
        pool,
        3,
        3,
        vec![
            Instruction::Aload_0,
            Instruction::Invokespecial(init),
            Instruction::Aload_0,
            Instruction::Lload_1,
            Instruction::Putfield(handle),
            Instruction::Return,
        ],
    )
}

fn load_callback_run(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class(LOAD_CALLBACK_CLASS)?;
    let handle = pool.add_field_ref(owner, "handle", "J")?;
    let native = pool.add_class(NATIVE_CLASS)?;
    let dispatch = pool.add_method_ref(native, "dispatchLoad", "(J)V")?;
    code(
        pool,
        2,
        1,
        vec![
            Instruction::Aload_0,
            Instruction::Getfield(handle),
            Instruction::Invokestatic(dispatch),
            Instruction::Return,
        ],
    )
}

fn manager_encode_track_details(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let native = pool.add_class(NATIVE_CLASS)?;
    let encode = pool.add_method_ref(
        native,
        "encodeTrackDetails",
        "(Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;)[B",
    )?;
    code(
        pool,
        1,
        2,
        vec![
            Instruction::Aload_1,
            Instruction::Invokestatic(encode),
            Instruction::Areturn,
        ],
    )
}

fn manager_decode_track_details(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let native = pool.add_class(NATIVE_CLASS)?;
    let decode = pool.add_method_ref(
        native,
        "decodeTrackDetails",
        "(Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackInfo;[B)Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;",
    )?;
    code(
        pool,
        2,
        3,
        vec![
            Instruction::Aload_1,
            Instruction::Aload_2,
            Instruction::Invokestatic(decode),
            Instruction::Areturn,
        ],
    )
}

fn clean_method(pool: &mut ConstantPool<'static>, owner: &str, field: &str) -> Result<Attribute> {
    let owner = pool.add_class(owner)?;
    let cleanable_field = pool.add_field_ref(owner, field, "Ljava/lang/ref/Cleaner$Cleanable;")?;
    let cleanable_class = pool.add_class("java/lang/ref/Cleaner$Cleanable")?;
    let clean = pool.add_interface_method_ref(cleanable_class, "clean", "()V")?;
    code(
        pool,
        1,
        1,
        vec![
            Instruction::Aload_0,
            Instruction::Getfield(cleanable_field),
            Instruction::Invokeinterface(clean, 1),
            Instruction::Return,
        ],
    )
}

fn state_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let object = pool.add_class("java/lang/Object")?;
    let init = pool.add_method_ref(object, "<init>", "()V")?;
    let state = pool.add_class(STATE_CLASS)?;
    let handle = pool.add_field_ref(state, "handle", "J")?;
    code(
        pool,
        3,
        3,
        vec![
            Instruction::Aload_0,
            Instruction::Invokespecial(init),
            Instruction::Aload_0,
            Instruction::Lload_1,
            Instruction::Putfield(handle),
            Instruction::Return,
        ],
    )
}

fn state_run(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let state = pool.add_class(STATE_CLASS)?;
    let handle = pool.add_field_ref(state, "handle", "J")?;
    let native = pool.add_class(NATIVE_CLASS)?;
    let release = pool.add_method_ref(native, "release", "(J)V")?;
    code(
        pool,
        2,
        1,
        vec![
            Instruction::Aload_0,
            Instruction::Getfield(handle),
            Instruction::Invokestatic(release),
            Instruction::Return,
        ],
    )
}

fn cleaner_clinit(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let cleaner_class = pool.add_class("java/lang/ref/Cleaner")?;
    let create = pool.add_method_ref(cleaner_class, "create", "()Ljava/lang/ref/Cleaner;")?;
    let owner = pool.add_class(CLEANER_CLASS)?;
    let cleaner = pool.add_field_ref(owner, "CLEANER", "Ljava/lang/ref/Cleaner;")?;
    code(
        pool,
        1,
        0,
        vec![
            Instruction::Invokestatic(create),
            Instruction::Putstatic(cleaner),
            Instruction::Return,
        ],
    )
}

fn cleaner_register(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class(CLEANER_CLASS)?;
    let cleaner = pool.add_field_ref(owner, "CLEANER", "Ljava/lang/ref/Cleaner;")?;
    let cleaner_class = pool.add_class("java/lang/ref/Cleaner")?;
    let register = pool.add_method_ref(
        cleaner_class,
        "register",
        "(Ljava/lang/Object;Ljava/lang/Runnable;)Ljava/lang/ref/Cleaner$Cleanable;",
    )?;
    let state = pool.add_class(STATE_CLASS)?;
    let init = pool.add_method_ref(state, "<init>", "(J)V")?;
    code(
        pool,
        6,
        3,
        vec![
            Instruction::Getstatic(cleaner),
            Instruction::Aload_0,
            Instruction::New(state),
            Instruction::Dup,
            Instruction::Lload_1,
            Instruction::Invokespecial(init),
            Instruction::Invokevirtual(register),
            Instruction::Areturn,
        ],
    )
}

fn probe_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let object = pool.add_class("java/lang/Object")?;
    let object_init = pool.add_method_ref(object, "<init>", "()V")?;
    let probe = pool.add_class(PROBE_CLASS)?;
    let handle = pool.add_field_ref(probe, "handle", "J")?;
    let cleanable = pool.add_field_ref(probe, "cleanable", "Ljava/lang/ref/Cleaner$Cleanable;")?;
    let native = pool.add_class(NATIVE_CLASS)?;
    let create = pool.add_method_ref(native, "createHandle", "(I)J")?;
    let cleaner = pool.add_class(CLEANER_CLASS)?;
    let register = pool.add_method_ref(
        cleaner,
        "register",
        "(Ljava/lang/Object;J)Ljava/lang/ref/Cleaner$Cleanable;",
    )?;
    code(
        pool,
        4,
        1,
        vec![
            Instruction::Aload_0,
            Instruction::Invokespecial(object_init),
            Instruction::Aload_0,
            Instruction::Iconst_5,
            Instruction::Invokestatic(create),
            Instruction::Putfield(handle),
            Instruction::Aload_0,
            Instruction::Aload_0,
            Instruction::Aload_0,
            Instruction::Getfield(handle),
            Instruction::Invokestatic(register),
            Instruction::Putfield(cleanable),
            Instruction::Return,
        ],
    )
}

fn invocation_handler_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let object = pool.add_class("java/lang/Object")?;
    let object_init = pool.add_method_ref(object, "<init>", "()V")?;
    let owner = pool.add_class(HANDLER_CLASS)?;
    let kind = pool.add_field_ref(owner, "kind", "I")?;
    let identifier = pool.add_field_ref(owner, "identifier", "Ljava/lang/String;")?;
    let listeners = pool.add_field_ref(owner, "listeners", "Ljava/util/ArrayList;")?;
    let volume = pool.add_field_ref(owner, "volume", "I")?;
    let handle = pool.add_field_ref(owner, "handle", "J")?;
    let cleanable = pool.add_field_ref(owner, "cleanable", "Ljava/lang/ref/Cleaner$Cleanable;")?;
    let list = pool.add_class("java/util/ArrayList")?;
    let list_init = pool.add_method_ref(list, "<init>", "()V")?;
    let native = pool.add_class(NATIVE_CLASS)?;
    let create = pool.add_method_ref(native, "createCoreHandle", "(ILjava/lang/String;)J")?;
    let cleaner = pool.add_class(CLEANER_CLASS)?;
    let register = pool.add_method_ref(
        cleaner,
        "register",
        "(Ljava/lang/Object;J)Ljava/lang/ref/Cleaner$Cleanable;",
    )?;
    code(
        pool,
        4,
        3,
        vec![
            Instruction::Aload_0,
            Instruction::Invokespecial(object_init),
            Instruction::Aload_0,
            Instruction::Iload_1,
            Instruction::Putfield(kind),
            Instruction::Aload_0,
            Instruction::Aload_2,
            Instruction::Putfield(identifier),
            Instruction::Aload_0,
            Instruction::New(list),
            Instruction::Dup,
            Instruction::Invokespecial(list_init),
            Instruction::Putfield(listeners),
            Instruction::Aload_0,
            Instruction::Bipush(100),
            Instruction::Putfield(volume),
            Instruction::Aload_0,
            Instruction::Iload_1,
            Instruction::Aload_2,
            Instruction::Invokestatic(create),
            Instruction::Putfield(handle),
            Instruction::Aload_0,
            Instruction::Aload_0,
            Instruction::Aload_0,
            Instruction::Getfield(handle),
            Instruction::Invokestatic(register),
            Instruction::Putfield(cleanable),
            Instruction::Return,
        ],
    )
}

fn getter_long(pool: &mut ConstantPool<'static>, owner: &str, field: &str) -> Result<Attribute> {
    let owner = pool.add_class(owner)?;
    let field = pool.add_field_ref(owner, field, "J")?;
    code(
        pool,
        2,
        1,
        vec![
            Instruction::Aload_0,
            Instruction::Getfield(field),
            Instruction::Lreturn,
        ],
    )
}

fn audio_event_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let object = pool.add_class("java/lang/Object")?;
    let init = pool.add_method_ref(object, "<init>", "()V")?;
    let owner = pool.add_class("com/sedmelluq/discord/lavaplayer/player/event/AudioEvent")?;
    let player = pool.add_field_ref(
        owner,
        "player",
        "Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;",
    )?;
    code(
        pool,
        2,
        2,
        vec![
            Instruction::Aload_0,
            Instruction::Invokespecial(init),
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Putfield(player),
            Instruction::Return,
        ],
    )
}

fn track_start_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let event = pool.add_class("com/sedmelluq/discord/lavaplayer/player/event/AudioEvent")?;
    let init = pool.add_method_ref(
        event,
        "<init>",
        "(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;)V",
    )?;
    let owner = pool.add_class("com/sedmelluq/discord/lavaplayer/player/event/TrackStartEvent")?;
    let track = pool.add_field_ref(
        owner,
        "track",
        "Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;",
    )?;
    code(
        pool,
        2,
        3,
        vec![
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Invokespecial(init),
            Instruction::Aload_0,
            Instruction::Aload_2,
            Instruction::Putfield(track),
            Instruction::Return,
        ],
    )
}

fn simple_audio_event_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let event = pool.add_class("com/sedmelluq/discord/lavaplayer/player/event/AudioEvent")?;
    let init = pool.add_method_ref(
        event,
        "<init>",
        "(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;)V",
    )?;
    code(
        pool,
        2,
        2,
        vec![
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Invokespecial(init),
            Instruction::Return,
        ],
    )
}

fn track_end_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let event = pool.add_class("com/sedmelluq/discord/lavaplayer/player/event/AudioEvent")?;
    let init = pool.add_method_ref(
        event,
        "<init>",
        "(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;)V",
    )?;
    let owner = pool.add_class("com/sedmelluq/discord/lavaplayer/player/event/TrackEndEvent")?;
    let track = pool.add_field_ref(
        owner,
        "track",
        "Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;",
    )?;
    let reason = pool.add_field_ref(
        owner,
        "endReason",
        "Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackEndReason;",
    )?;
    code(
        pool,
        2,
        4,
        vec![
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Invokespecial(init),
            Instruction::Aload_0,
            Instruction::Aload_2,
            Instruction::Putfield(track),
            Instruction::Aload_0,
            Instruction::Aload_3,
            Instruction::Putfield(reason),
            Instruction::Return,
        ],
    )
}

fn track_exception_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let event = pool.add_class("com/sedmelluq/discord/lavaplayer/player/event/AudioEvent")?;
    let init = pool.add_method_ref(
        event,
        "<init>",
        "(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;)V",
    )?;
    let owner = pool.add_class(TRACK_EXCEPTION_EVENT_CLASS)?;
    let track = pool.add_field_ref(
        owner,
        "track",
        "Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;",
    )?;
    let exception = pool.add_field_ref(
        owner,
        "exception",
        "Lcom/sedmelluq/discord/lavaplayer/tools/FriendlyException;",
    )?;
    code(
        pool,
        2,
        4,
        vec![
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Invokespecial(init),
            Instruction::Aload_0,
            Instruction::Aload_2,
            Instruction::Putfield(track),
            Instruction::Aload_0,
            Instruction::Aload_3,
            Instruction::Putfield(exception),
            Instruction::Return,
        ],
    )
}

fn track_stuck_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let event = pool.add_class("com/sedmelluq/discord/lavaplayer/player/event/AudioEvent")?;
    let init = pool.add_method_ref(
        event,
        "<init>",
        "(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;)V",
    )?;
    let owner = pool.add_class(TRACK_STUCK_EVENT_CLASS)?;
    let track = pool.add_field_ref(
        owner,
        "track",
        "Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;",
    )?;
    let threshold = pool.add_field_ref(owner, "thresholdMs", "J")?;
    let stack_trace = pool.add_field_ref(owner, "stackTrace", "[Ljava/lang/StackTraceElement;")?;
    code(
        pool,
        3,
        6,
        vec![
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Invokespecial(init),
            Instruction::Aload_0,
            Instruction::Aload_2,
            Instruction::Putfield(track),
            Instruction::Aload_0,
            Instruction::Lload_3,
            Instruction::Putfield(threshold),
            Instruction::Aload_0,
            Instruction::Aload(5),
            Instruction::Putfield(stack_trace),
            Instruction::Return,
        ],
    )
}

fn event_adapter_stuck_with_trace(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class(EVENT_ADAPTER_CLASS)?;
    let callback = pool.add_method_ref(
        owner,
        "onTrackStuck",
        "(Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;J)V",
    )?;
    code(
        pool,
        5,
        6,
        vec![
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Aload_2,
            Instruction::Lload_3,
            Instruction::Invokevirtual(callback),
            Instruction::Return,
        ],
    )
}

fn event_adapter_dispatch(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let dispatcher = pool.add_class(EVENT_DISPATCHER_CLASS)?;
    let dispatch = pool.add_method_ref(
        dispatcher,
        "dispatch",
        "(Lcom/sedmelluq/discord/lavaplayer/player/event/AudioEventAdapter;Lcom/sedmelluq/discord/lavaplayer/player/event/AudioEvent;)V",
    )?;
    code(
        pool,
        2,
        2,
        vec![
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Invokestatic(dispatch),
            Instruction::Return,
        ],
    )
}

// Keeping the dispatch bytecode linear makes its branch targets directly auditable against javap.
#[allow(clippy::too_many_lines)]
fn event_dispatch_body(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    const PLAYER_DESCRIPTOR: &str = "Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;";
    const TRACK_DESCRIPTOR: &str = "Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;";
    let event = pool.add_class("com/sedmelluq/discord/lavaplayer/player/event/AudioEvent")?;
    let player = pool.add_field_ref(event, "player", PLAYER_DESCRIPTOR)?;
    let pause = pool.add_class("com/sedmelluq/discord/lavaplayer/player/event/PlayerPauseEvent")?;
    let resume =
        pool.add_class("com/sedmelluq/discord/lavaplayer/player/event/PlayerResumeEvent")?;
    let start = pool.add_class("com/sedmelluq/discord/lavaplayer/player/event/TrackStartEvent")?;
    let end = pool.add_class("com/sedmelluq/discord/lavaplayer/player/event/TrackEndEvent")?;
    let exception = pool.add_class(TRACK_EXCEPTION_EVENT_CLASS)?;
    let stuck = pool.add_class(TRACK_STUCK_EVENT_CLASS)?;
    let start_track = pool.add_field_ref(start, "track", TRACK_DESCRIPTOR)?;
    let end_track = pool.add_field_ref(end, "track", TRACK_DESCRIPTOR)?;
    let end_reason = pool.add_field_ref(
        end,
        "endReason",
        "Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackEndReason;",
    )?;
    let exception_track = pool.add_field_ref(exception, "track", TRACK_DESCRIPTOR)?;
    let exception_value = pool.add_field_ref(
        exception,
        "exception",
        "Lcom/sedmelluq/discord/lavaplayer/tools/FriendlyException;",
    )?;
    let stuck_track = pool.add_field_ref(stuck, "track", TRACK_DESCRIPTOR)?;
    let stuck_threshold = pool.add_field_ref(stuck, "thresholdMs", "J")?;
    let stuck_stack_trace =
        pool.add_field_ref(stuck, "stackTrace", "[Ljava/lang/StackTraceElement;")?;
    let adapter = pool.add_class(EVENT_ADAPTER_CLASS)?;
    let on_pause =
        pool.add_method_ref(adapter, "onPlayerPause", &format!("({PLAYER_DESCRIPTOR})V"))?;
    let on_resume = pool.add_method_ref(
        adapter,
        "onPlayerResume",
        &format!("({PLAYER_DESCRIPTOR})V"),
    )?;
    let on_start = pool.add_method_ref(
        adapter,
        "onTrackStart",
        &format!("({PLAYER_DESCRIPTOR}{TRACK_DESCRIPTOR})V"),
    )?;
    let on_end = pool.add_method_ref(
        adapter,
        "onTrackEnd",
        &format!(
            "({PLAYER_DESCRIPTOR}{TRACK_DESCRIPTOR}Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackEndReason;)V"
        ),
    )?;
    let on_exception = pool.add_method_ref(
        adapter,
        "onTrackException",
        &format!(
            "({PLAYER_DESCRIPTOR}{TRACK_DESCRIPTOR}Lcom/sedmelluq/discord/lavaplayer/tools/FriendlyException;)V"
        ),
    )?;
    let on_stuck = pool.add_method_ref(
        adapter,
        "onTrackStuck",
        &format!("({PLAYER_DESCRIPTOR}{TRACK_DESCRIPTOR}J[Ljava/lang/StackTraceElement;)V"),
    )?;
    code(
        pool,
        6,
        2,
        vec![
            Instruction::Aload_1,
            Instruction::Instanceof(pause),
            Instruction::Ifeq(8),
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Getfield(player),
            Instruction::Invokevirtual(on_pause),
            Instruction::Goto(71),
            Instruction::Aload_1,
            Instruction::Instanceof(resume),
            Instruction::Ifeq(16),
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Getfield(player),
            Instruction::Invokevirtual(on_resume),
            Instruction::Goto(71),
            Instruction::Aload_1,
            Instruction::Instanceof(start),
            Instruction::Ifeq(27),
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Getfield(player),
            Instruction::Aload_1,
            Instruction::Checkcast(start),
            Instruction::Getfield(start_track),
            Instruction::Invokevirtual(on_start),
            Instruction::Goto(71),
            Instruction::Aload_1,
            Instruction::Instanceof(end),
            Instruction::Ifeq(41),
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Getfield(player),
            Instruction::Aload_1,
            Instruction::Checkcast(end),
            Instruction::Getfield(end_track),
            Instruction::Aload_1,
            Instruction::Checkcast(end),
            Instruction::Getfield(end_reason),
            Instruction::Invokevirtual(on_end),
            Instruction::Goto(71),
            Instruction::Aload_1,
            Instruction::Instanceof(exception),
            Instruction::Ifeq(55),
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Getfield(player),
            Instruction::Aload_1,
            Instruction::Checkcast(exception),
            Instruction::Getfield(exception_track),
            Instruction::Aload_1,
            Instruction::Checkcast(exception),
            Instruction::Getfield(exception_value),
            Instruction::Invokevirtual(on_exception),
            Instruction::Goto(71),
            Instruction::Aload_1,
            Instruction::Instanceof(stuck),
            Instruction::Ifeq(71),
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Getfield(player),
            Instruction::Aload_1,
            Instruction::Checkcast(stuck),
            Instruction::Getfield(stuck_track),
            Instruction::Aload_1,
            Instruction::Checkcast(stuck),
            Instruction::Getfield(stuck_threshold),
            Instruction::Aload_1,
            Instruction::Checkcast(stuck),
            Instruction::Getfield(stuck_stack_trace),
            Instruction::Invokevirtual(on_stuck),
            Instruction::Return,
        ],
    )
}

fn track_info_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let object = pool.add_class("java/lang/Object")?;
    let init = pool.add_method_ref(object, "<init>", "()V")?;
    let owner = pool.add_class("com/sedmelluq/discord/lavaplayer/track/AudioTrackInfo")?;
    let title = pool.add_field_ref(owner, "title", "Ljava/lang/String;")?;
    let author = pool.add_field_ref(owner, "author", "Ljava/lang/String;")?;
    let length = pool.add_field_ref(owner, "length", "J")?;
    let identifier = pool.add_field_ref(owner, "identifier", "Ljava/lang/String;")?;
    let stream = pool.add_field_ref(owner, "isStream", "Z")?;
    let uri = pool.add_field_ref(owner, "uri", "Ljava/lang/String;")?;
    let artwork = pool.add_field_ref(owner, "artworkUrl", "Ljava/lang/String;")?;
    let isrc = pool.add_field_ref(owner, "isrc", "Ljava/lang/String;")?;
    code(
        pool,
        3,
        10,
        vec![
            Instruction::Aload_0,
            Instruction::Invokespecial(init),
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Putfield(title),
            Instruction::Aload_0,
            Instruction::Aload_2,
            Instruction::Putfield(author),
            Instruction::Aload_0,
            Instruction::Lload_3,
            Instruction::Putfield(length),
            Instruction::Aload_0,
            Instruction::Aload(5),
            Instruction::Putfield(identifier),
            Instruction::Aload_0,
            Instruction::Iload(6),
            Instruction::Putfield(stream),
            Instruction::Aload_0,
            Instruction::Aload(7),
            Instruction::Putfield(uri),
            Instruction::Aload_0,
            Instruction::Aload(8),
            Instruction::Putfield(artwork),
            Instruction::Aload_0,
            Instruction::Aload(9),
            Instruction::Putfield(isrc),
            Instruction::Return,
        ],
    )
}

fn track_info_short_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class("com/sedmelluq/discord/lavaplayer/track/AudioTrackInfo")?;
    let init = pool.add_method_ref(
        owner,
        "<init>",
        "(Ljava/lang/String;Ljava/lang/String;JLjava/lang/String;ZLjava/lang/String;Ljava/lang/String;Ljava/lang/String;)V",
    )?;
    code(
        pool,
        10,
        8,
        vec![
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Aload_2,
            Instruction::Lload_3,
            Instruction::Aload(5),
            Instruction::Iload(6),
            Instruction::Aload(7),
            Instruction::Aconst_null,
            Instruction::Aconst_null,
            Instruction::Invokespecial(init),
            Instruction::Return,
        ],
    )
}

fn audio_configuration_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let object = pool.add_class("java/lang/Object")?;
    let object_init = pool.add_method_ref(object, "<init>", "()V")?;
    let owner = pool.add_class(CONFIGURATION_CLASS)?;
    let quality_field = pool.add_field_ref(
        owner,
        "resamplingQuality",
        "Lcom/sedmelluq/discord/lavaplayer/player/AudioConfiguration$ResamplingQuality;",
    )?;
    let opus_quality = pool.add_field_ref(owner, "opusEncodingQuality", "I")?;
    let output = pool.add_field_ref(
        owner,
        "outputFormat",
        "Lcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;",
    )?;
    let frame_buffer_factory = pool.add_field_ref(
        owner,
        "frameBufferFactory",
        "Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioFrameBufferFactory;",
    )?;
    let quality = pool.add_class(RESAMPLING_CLASS)?;
    let low = pool.add_field_ref(
        quality,
        "LOW",
        "Lcom/sedmelluq/discord/lavaplayer/player/AudioConfiguration$ResamplingQuality;",
    )?;
    let format = pool.add_class(FORMAT_CLASS)?;
    let format_init = pool.add_method_ref(format, "<init>", "(III)V")?;
    let factory = pool.add_class(FRAME_BUFFER_FACTORY_CLASS)?;
    let factory_init = pool.add_method_ref(factory, "<init>", "()V")?;
    let sample_rate = pool.add_integer(48_000)?;
    code(
        pool,
        6,
        1,
        vec![
            Instruction::Aload_0,
            Instruction::Invokespecial(object_init),
            Instruction::Aload_0,
            Instruction::Getstatic(low),
            Instruction::Putfield(quality_field),
            Instruction::Aload_0,
            Instruction::Bipush(10),
            Instruction::Putfield(opus_quality),
            Instruction::Aload_0,
            Instruction::New(format),
            Instruction::Dup,
            Instruction::Iconst_2,
            Instruction::Ldc_w(sample_rate),
            Instruction::Sipush(960),
            Instruction::Invokespecial(format_init),
            Instruction::Putfield(output),
            Instruction::Aload_0,
            Instruction::New(factory),
            Instruction::Dup,
            Instruction::Invokespecial(factory_init),
            Instruction::Putfield(frame_buffer_factory),
            Instruction::Return,
        ],
    )
}

fn audio_configuration_set_opus_quality(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class(CONFIGURATION_CLASS)?;
    let field = pool.add_field_ref(owner, "opusEncodingQuality", "I")?;
    let math = pool.add_class("java/lang/Math")?;
    let min = pool.add_method_ref(math, "min", "(II)I")?;
    let max = pool.add_method_ref(math, "max", "(II)I")?;
    code(
        pool,
        4,
        2,
        vec![
            Instruction::Aload_0,
            Instruction::Iconst_0,
            Instruction::Iload_1,
            Instruction::Bipush(10),
            Instruction::Invokestatic(min),
            Instruction::Invokestatic(max),
            Instruction::Putfield(field),
            Instruction::Return,
        ],
    )
}

fn audio_configuration_copy(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let owner = pool.add_class(CONFIGURATION_CLASS)?;
    let init = pool.add_method_ref(owner, "<init>", "()V")?;
    let fields = [
        (
            "resamplingQuality",
            "Lcom/sedmelluq/discord/lavaplayer/player/AudioConfiguration$ResamplingQuality;",
        ),
        ("opusEncodingQuality", "I"),
        (
            "outputFormat",
            "Lcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;",
        ),
        ("filterHotSwapEnabled", "Z"),
        (
            "frameBufferFactory",
            "Lcom/sedmelluq/discord/lavaplayer/track/playback/AudioFrameBufferFactory;",
        ),
    ];
    let field_refs = fields
        .iter()
        .map(|(name, descriptor)| pool.add_field_ref(owner, *name, *descriptor))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut instructions = vec![
        Instruction::New(owner),
        Instruction::Dup,
        Instruction::Invokespecial(init),
        Instruction::Astore_1,
    ];
    for field in field_refs {
        instructions.extend([
            Instruction::Aload_1,
            Instruction::Aload_0,
            Instruction::Getfield(field),
            Instruction::Putfield(field),
        ]);
    }
    instructions.extend([Instruction::Aload_1, Instruction::Areturn]);
    code(pool, 3, 2, instructions)
}

fn audio_data_format_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let object = pool.add_class("java/lang/Object")?;
    let init = pool.add_method_ref(object, "<init>", "()V")?;
    let owner = pool.add_class("com/sedmelluq/discord/lavaplayer/format/AudioDataFormat")?;
    let channels = pool.add_field_ref(owner, "channelCount", "I")?;
    let sample_rate = pool.add_field_ref(owner, "sampleRate", "I")?;
    let chunk_samples = pool.add_field_ref(owner, "chunkSampleCount", "I")?;
    code(
        pool,
        2,
        4,
        vec![
            Instruction::Aload_0,
            Instruction::Invokespecial(init),
            Instruction::Aload_0,
            Instruction::Iload_1,
            Instruction::Putfield(channels),
            Instruction::Aload_0,
            Instruction::Iload_2,
            Instruction::Putfield(sample_rate),
            Instruction::Aload_0,
            Instruction::Iload_3,
            Instruction::Putfield(chunk_samples),
            Instruction::Return,
        ],
    )
}

fn native_audio_data_format_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let parent = pool.add_class("com/sedmelluq/discord/lavaplayer/format/AudioDataFormat")?;
    let init = pool.add_method_ref(parent, "<init>", "(III)V")?;
    code(
        pool,
        4,
        4,
        vec![
            Instruction::Aload_0,
            Instruction::Iload_1,
            Instruction::Iload_2,
            Instruction::Iload_3,
            Instruction::Invokespecial(init),
            Instruction::Return,
        ],
    )
}

fn track_marker_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let object = pool.add_class("java/lang/Object")?;
    let init = pool.add_method_ref(object, "<init>", "()V")?;
    let owner = pool.add_class("com/sedmelluq/discord/lavaplayer/track/TrackMarker")?;
    let timecode = pool.add_field_ref(owner, "timecode", "J")?;
    let handler = pool.add_field_ref(
        owner,
        "handler",
        "Lcom/sedmelluq/discord/lavaplayer/track/TrackMarkerHandler;",
    )?;
    code(
        pool,
        3,
        4,
        vec![
            Instruction::Aload_0,
            Instruction::Invokespecial(init),
            Instruction::Aload_0,
            Instruction::Lload_1,
            Instruction::Putfield(timecode),
            Instruction::Aload_0,
            Instruction::Aload_3,
            Instruction::Putfield(handler),
            Instruction::Return,
        ],
    )
}

fn marker_state_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let enumeration = pool.add_class("java/lang/Enum")?;
    let init = pool.add_method_ref(enumeration, "<init>", "(Ljava/lang/String;I)V")?;
    code(
        pool,
        3,
        3,
        vec![
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Iload_2,
            Instruction::Invokespecial(init),
            Instruction::Return,
        ],
    )
}

fn enum_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    marker_state_constructor(pool)
}

fn end_reason_constructor(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let enumeration = pool.add_class("java/lang/Enum")?;
    let init = pool.add_method_ref(enumeration, "<init>", "(Ljava/lang/String;I)V")?;
    let owner = pool.add_class("com/sedmelluq/discord/lavaplayer/track/AudioTrackEndReason")?;
    let may_start_next = pool.add_field_ref(owner, "mayStartNext", "Z")?;
    code(
        pool,
        3,
        4,
        vec![
            Instruction::Aload_0,
            Instruction::Aload_1,
            Instruction::Iload_2,
            Instruction::Invokespecial(init),
            Instruction::Aload_0,
            Instruction::Iload_3,
            Instruction::Putfield(may_start_next),
            Instruction::Return,
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_body_has_success_and_failure_paths() -> Result<()> {
        let mut pool = ConstantPool::new();
        let Attribute::Code { code, .. } = ensure_abi_body(&mut pool, 1)? else {
            return Err("expected Code attribute".into());
        };
        assert!(matches!(code.get(2), Some(Instruction::If_icmpeq(8))));
        assert!(matches!(code.last(), Some(Instruction::Return)));
        Ok(())
    }
}
