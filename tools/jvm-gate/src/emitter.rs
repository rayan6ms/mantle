use std::error::Error;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

use ristretto_classfile::attributes::{Attribute, Instruction};
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
const LOADER_CLASS: &str = "dev/mantle/internal/NativeLoader";
const FORMAT_CLASS: &str = "dev/mantle/internal/NativeAudioDataFormat";
const MANAGER_CLASS: &str = "com/sedmelluq/discord/lavaplayer/player/DefaultAudioPlayerManager";
const CONFIGURATION_CLASS: &str = "com/sedmelluq/discord/lavaplayer/player/AudioConfiguration";
const RESAMPLING_CLASS: &str =
    "com/sedmelluq/discord/lavaplayer/player/AudioConfiguration$ResamplingQuality";
const MARKER_STATE_CLASS: &str =
    "com/sedmelluq/discord/lavaplayer/track/TrackMarkerHandler$MarkerState";

const REFERENCE_CLASSES: &[&str] = &[
    "com/sedmelluq/discord/lavaplayer/player/AudioLoadResultHandler",
    CONFIGURATION_CLASS,
    RESAMPLING_CLASS,
    "com/sedmelluq/discord/lavaplayer/player/AudioPlayer",
    "com/sedmelluq/discord/lavaplayer/player/AudioPlayerManager",
    MANAGER_CLASS,
    "com/sedmelluq/discord/lavaplayer/player/event/AudioEvent",
    "com/sedmelluq/discord/lavaplayer/player/event/AudioEventListener",
    "com/sedmelluq/discord/lavaplayer/player/event/PlayerPauseEvent",
    "com/sedmelluq/discord/lavaplayer/player/event/PlayerResumeEvent",
    "com/sedmelluq/discord/lavaplayer/player/event/TrackEndEvent",
    "com/sedmelluq/discord/lavaplayer/player/event/TrackStartEvent",
    "com/sedmelluq/discord/lavaplayer/filter/PcmFilterFactory",
    "com/sedmelluq/discord/lavaplayer/format/AudioDataFormat",
    "com/sedmelluq/discord/lavaplayer/source/AudioSourceManager",
    "com/sedmelluq/discord/lavaplayer/tools/FriendlyException",
    "com/sedmelluq/discord/lavaplayer/track/AudioItem",
    "com/sedmelluq/discord/lavaplayer/track/AudioPlaylist",
    "com/sedmelluq/discord/lavaplayer/track/AudioTrack",
    "com/sedmelluq/discord/lavaplayer/track/AudioTrackEndReason",
    "com/sedmelluq/discord/lavaplayer/track/AudioTrackInfo",
    "com/sedmelluq/discord/lavaplayer/track/AudioTrackState",
    "com/sedmelluq/discord/lavaplayer/track/TrackMarker",
    "com/sedmelluq/discord/lavaplayer/track/TrackMarkerHandler",
    "com/sedmelluq/discord/lavaplayer/track/TrackMarkerHandler$MarkerState",
    "com/sedmelluq/discord/lavaplayer/track/playback/AudioFrame",
    "com/sedmelluq/discord/lavaplayer/track/playback/AudioFrameProvider",
    "com/sedmelluq/discord/lavaplayer/track/playback/AbstractMutableAudioFrame",
    "com/sedmelluq/discord/lavaplayer/track/playback/ImmutableAudioFrame",
    "com/sedmelluq/discord/lavaplayer/track/playback/MutableAudioFrame",
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
    classes.extend([
        native_class(expected_abi)?,
        native_state_class()?,
        native_cleaner_class()?,
        native_probe_class()?,
        native_invocation_handler_class()?,
        native_loader_class()?,
        native_audio_data_format_class()?,
    ]);

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
        field
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
    Ok(match (class_name, name, descriptor) {
        (MANAGER_CLASS, "<init>", "()V") => manager_constructor(pool)?,
        (MANAGER_CLASS, "shutdown", "()V") => manager_shutdown(pool)?,
        (
            MANAGER_CLASS,
            "getConfiguration",
            "()Lcom/sedmelluq/discord/lavaplayer/player/AudioConfiguration;",
        ) => manager_configuration(pool)?,
        (MANAGER_CLASS, "getFrameBufferDuration", "()I") => integer_return(pool, 5_000, 1)?,
        (MANAGER_CLASS, "isUsingSeekGhosting", "()Z") => boolean_return(pool, true, 1)?,
        (
            MANAGER_CLASS,
            "createPlayer" | "constructPlayer",
            "()Lcom/sedmelluq/discord/lavaplayer/player/AudioPlayer;",
        ) => manager_create_player(pool)?,
        (
            MANAGER_CLASS,
            "encodeTrackDetails",
            "(Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;)[B",
        ) => manager_encode_track_details(pool)?,
        (
            MANAGER_CLASS,
            "decodeTrackDetails",
            "(Lcom/sedmelluq/discord/lavaplayer/track/AudioTrackInfo;[B)Lcom/sedmelluq/discord/lavaplayer/track/AudioTrack;",
        ) => manager_decode_track_details(pool)?,
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
        (CONFIGURATION_CLASS, "<init>", "()V") => audio_configuration_constructor(pool)?,
        (
            CONFIGURATION_CLASS,
            "getResamplingQuality",
            "()Lcom/sedmelluq/discord/lavaplayer/player/AudioConfiguration$ResamplingQuality;",
        ) => object_getter(
            pool,
            CONFIGURATION_CLASS,
            "resamplingQuality",
            "Lcom/sedmelluq/discord/lavaplayer/player/AudioConfiguration$ResamplingQuality;",
        )?,
        (CONFIGURATION_CLASS, "getOpusEncodingQuality", "()I") => {
            int_getter(pool, CONFIGURATION_CLASS, "opusEncodingQuality")?
        }
        (
            CONFIGURATION_CLASS,
            "getOutputFormat",
            "()Lcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;",
        ) => object_getter(
            pool,
            CONFIGURATION_CLASS,
            "outputFormat",
            "Lcom/sedmelluq/discord/lavaplayer/format/AudioDataFormat;",
        )?,
        (CONFIGURATION_CLASS, "isFilterHotSwapEnabled", "()Z") => {
            bool_getter(pool, CONFIGURATION_CLASS, "filterHotSwapEnabled")?
        }
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

fn add_reference_implementation_state(
    class: &mut ClassFile<'static>,
    class_name: &str,
) -> Result<()> {
    if class_name == MARKER_STATE_CLASS {
        let body = marker_state_constructor(&mut class.constant_pool)?;
        add_method(
            class,
            MethodAccessFlags::PRIVATE,
            "<init>",
            "(Ljava/lang/String;I)V",
            Some(body),
        )?;
    }
    if class_name == "com/sedmelluq/discord/lavaplayer/track/AudioTrackState" {
        let body = enum_constructor(&mut class.constant_pool)?;
        add_method(
            class,
            MethodAccessFlags::PRIVATE,
            "<init>",
            "(Ljava/lang/String;I)V",
            Some(body),
        )?;
    }
    if class_name == "com/sedmelluq/discord/lavaplayer/track/AudioTrackEndReason" {
        let body = end_reason_constructor(&mut class.constant_pool)?;
        add_method(
            class,
            MethodAccessFlags::PRIVATE,
            "<init>",
            "(Ljava/lang/String;IZ)V",
            Some(body),
        )?;
    }
    if class_name == RESAMPLING_CLASS {
        let body = enum_constructor(&mut class.constant_pool)?;
        add_method(
            class,
            MethodAccessFlags::PRIVATE,
            "<init>",
            "(Ljava/lang/String;I)V",
            Some(body),
        )?;
    }
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
        ] {
            add_field(class, FieldAccessFlags::PRIVATE, name, descriptor)?;
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
    let codec = string_return(&mut class.constant_pool, "opus", 1)?;
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
    Ok(Attribute::Code {
        name_index: pool.add_utf8("Code")?,
        max_stack,
        max_locals,
        code: instructions,
        exception_table: Vec::new(),
        attributes: Vec::new(),
    })
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

fn bool_getter(pool: &mut ConstantPool<'static>, owner: &str, field: &str) -> Result<Attribute> {
    primitive_getter(pool, owner, field, "Z")
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

fn manager_load_ordered_string(pool: &mut ConstantPool<'static>) -> Result<Attribute> {
    let native = pool.add_class(NATIVE_CLASS)?;
    let load = pool.add_method_ref(
        native,
        "loadItem",
        "(Ljava/lang/String;Lcom/sedmelluq/discord/lavaplayer/player/AudioLoadResultHandler;)Ljava/util/concurrent/Future;",
    )?;
    code(
        pool,
        2,
        4,
        vec![
            Instruction::Aload_2,
            Instruction::Aload_3,
            Instruction::Invokestatic(load),
            Instruction::Areturn,
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
    let quality = pool.add_class(RESAMPLING_CLASS)?;
    let quality_init = pool.add_method_ref(quality, "<init>", "(Ljava/lang/String;I)V")?;
    let quality_name = pool.add_string("LOW")?;
    let format = pool.add_class(FORMAT_CLASS)?;
    let format_init = pool.add_method_ref(format, "<init>", "(III)V")?;
    let sample_rate = pool.add_integer(48_000)?;
    code(
        pool,
        6,
        1,
        vec![
            Instruction::Aload_0,
            Instruction::Invokespecial(object_init),
            Instruction::Aload_0,
            Instruction::New(quality),
            Instruction::Dup,
            Instruction::Ldc_w(quality_name),
            Instruction::Iconst_2,
            Instruction::Invokespecial(quality_init),
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
            Instruction::Return,
        ],
    )
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
