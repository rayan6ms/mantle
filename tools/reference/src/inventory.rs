use std::collections::BTreeSet;
use std::error::Error;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use ristretto_classfile::attributes::{Annotation, Attribute, InnerClass, TypeAnnotation};
use ristretto_classfile::{
    ClassAccessFlags, ClassFile, Constant, FieldAccessFlags, MethodAccessFlags,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const LAVAPLAYER_PREFIX: &str = "com/sedmelluq/discord/lavaplayer/";
const SOURCE_MANAGERS_PATH: &str =
    "com/sedmelluq/discord/lavaplayer/source/AudioSourceManagers.java";

#[derive(Serialize)]
struct Inventory {
    schema_version: u32,
    reference: Reference,
    counts: Counts,
    classes: Vec<ClassContract>,
    external_public_types: Vec<String>,
    resources: Vec<Resource>,
    pom_dependencies: Vec<PomDependency>,
    gradle_module_metadata: serde_json::Value,
    built_in_sources: BuiltInSources,
}

#[derive(Serialize)]
struct Reference {
    artifact: &'static str,
    version: &'static str,
    sha256: String,
    coordinate: &'static str,
    jar_sha256: String,
    sources_jar_sha256: String,
    pom_sha256: String,
    module_metadata_sha256: String,
}

#[derive(Serialize)]
struct Counts {
    jar_entries: usize,
    class_entries: usize,
    exported_classes: usize,
    exported_fields: usize,
    exported_methods: usize,
    non_class_resources: usize,
    service_provider_files: usize,
    pom_dependencies: usize,
    external_public_types: usize,
}

#[derive(Serialize)]
struct ClassContract {
    binary_name: String,
    classfile_major: u16,
    classfile_minor: u16,
    access_flags: u16,
    access: String,
    visibility: &'static str,
    superclass: Option<String>,
    interfaces: Vec<String>,
    generic_signature: Option<String>,
    annotations: Vec<AnnotationContract>,
    attributes: Vec<String>,
    inner_classes: Vec<InnerClassContract>,
    fields: Vec<FieldContract>,
    methods: Vec<MethodContract>,
}

#[derive(Serialize)]
struct FieldContract {
    name: String,
    descriptor: String,
    access_flags: u16,
    access: String,
    generic_signature: Option<String>,
    annotations: Vec<AnnotationContract>,
    attributes: Vec<String>,
    constant_value: Option<String>,
    enum_constant: bool,
    synthetic: bool,
}

#[derive(Serialize)]
struct MethodContract {
    name: String,
    descriptor: String,
    access_flags: u16,
    access: String,
    generic_signature: Option<String>,
    annotations: Vec<AnnotationContract>,
    attributes: Vec<String>,
    checked_exceptions: Vec<String>,
    annotation_default: Option<String>,
    default_interface_method: bool,
    bridge: bool,
    synthetic: bool,
}

#[derive(Serialize)]
struct AnnotationContract {
    descriptor: String,
    visible: bool,
    target: &'static str,
    values: String,
}

#[derive(Serialize)]
struct InnerClassContract {
    binary_name: String,
    outer_binary_name: Option<String>,
    inner_name: Option<String>,
    access_flags: u16,
    access: String,
}

#[derive(Serialize)]
struct Resource {
    path: String,
    kind: &'static str,
    size: u64,
    compressed_size: u64,
    crc32: u32,
    sha256: String,
    utf8_content: Option<String>,
}

#[derive(Serialize)]
struct PomDependency {
    group_id: String,
    artifact_id: String,
    version: String,
    scope: String,
    optional: Option<String>,
    classifier: Option<String>,
    dependency_type: Option<String>,
}

#[derive(Serialize)]
struct BuiltInSources {
    evidence_source: &'static str,
    remote_registration_order: Vec<String>,
    local_registration_order: Vec<String>,
}

#[derive(Serialize)]
struct ClassificationSeed {
    schema_version: u32,
    reference: serde_json::Value,
    status: &'static str,
    symbols: Vec<SymbolAssessment>,
}

#[derive(Serialize)]
struct SymbolAssessment {
    binary_name: String,
    symbol_kind: &'static str,
    member_name: Option<String>,
    descriptor: Option<String>,
    assessment: &'static str,
    notes: &'static str,
    tests: Vec<String>,
}

pub fn run(args: &[String]) -> Result<()> {
    let jar = required_path(args, "--jar")?;
    let sources_jar = required_path(args, "--sources-jar")?;
    let pom = required_path(args, "--pom")?;
    let module = required_path(args, "--module")?;
    let output = required_path(args, "--output")?;

    let (mut classes, resources, jar_entries, class_entries) = inventory_jar(&jar)?;
    classes.sort_by(|left, right| left.binary_name.cmp(&right.binary_name));
    let external_public_types = external_types(&classes);
    let pom_dependencies = parse_pom_dependencies(&fs::read_to_string(&pom)?)?;
    let gradle_module_metadata = serde_json::from_slice(&fs::read(&module)?)?;
    let built_in_sources = inventory_built_in_sources(&sources_jar)?;
    validate_built_in_sources(&built_in_sources, &classes)?;

    let counts = Counts {
        jar_entries,
        class_entries,
        exported_classes: classes.len(),
        exported_fields: classes.iter().map(|class| class.fields.len()).sum(),
        exported_methods: classes.iter().map(|class| class.methods.len()).sum(),
        non_class_resources: resources.len(),
        service_provider_files: resources
            .iter()
            .filter(|resource| resource.kind == "service-provider")
            .count(),
        pom_dependencies: pom_dependencies.len(),
        external_public_types: external_public_types.len(),
    };
    let inventory = Inventory {
        schema_version: 1,
        reference: Reference {
            artifact: "dev.arbjerg:lavaplayer",
            version: "2.2.6",
            sha256: sha256_file(&jar)?,
            coordinate: "dev.arbjerg:lavaplayer:2.2.6",
            jar_sha256: sha256_file(&jar)?,
            sources_jar_sha256: sha256_file(&sources_jar)?,
            pom_sha256: sha256_file(&pom)?,
            module_metadata_sha256: sha256_file(&module)?,
        },
        counts,
        classes,
        external_public_types,
        resources,
        pom_dependencies,
        gradle_module_metadata,
        built_in_sources,
    };
    let mut encoded = serde_json::to_vec_pretty(&inventory)?;
    encoded.push(b'\n');
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, encoded)?;
    Ok(())
}

pub fn seed_classification(args: &[String]) -> Result<()> {
    let inventory_path = required_path(args, "--inventory")?;
    let output = required_path(args, "--output")?;
    let inventory: serde_json::Value = serde_json::from_slice(&fs::read(inventory_path)?)?;
    let reference = inventory
        .get("reference")
        .cloned()
        .ok_or("inventory has no reference object")?;
    let classes = inventory
        .get("classes")
        .and_then(serde_json::Value::as_array)
        .ok_or("inventory has no classes array")?;
    let mut symbols = Vec::new();
    for class in classes {
        let binary_name = json_string(class, "binary_name")?;
        symbols.push(unassessed_symbol(&binary_name, "CLASS", None, None));
        for field in json_array(class, "fields")? {
            symbols.push(unassessed_symbol(
                &binary_name,
                "FIELD",
                Some(json_string(field, "name")?),
                Some(json_string(field, "descriptor")?),
            ));
        }
        for method in json_array(class, "methods")? {
            symbols.push(unassessed_symbol(
                &binary_name,
                "METHOD",
                Some(json_string(method, "name")?),
                Some(json_string(method, "descriptor")?),
            ));
        }
    }
    let seed = ClassificationSeed {
        schema_version: 1,
        reference,
        status: "INITIAL_UNASSESSED",
        symbols,
    };
    let mut encoded = serde_json::to_vec_pretty(&seed)?;
    encoded.push(b'\n');
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, encoded)?;
    Ok(())
}

fn json_string(value: &serde_json::Value, key: &str) -> Result<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("inventory value has no string {key}").into())
}

fn json_array<'a>(value: &'a serde_json::Value, key: &str) -> Result<&'a [serde_json::Value]> {
    value
        .get(key)
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("inventory value has no array {key}").into())
}

fn unassessed_symbol(
    binary_name: &str,
    symbol_kind: &'static str,
    member_name: Option<String>,
    descriptor: Option<String>,
) -> SymbolAssessment {
    SymbolAssessment {
        binary_name: binary_name.to_owned(),
        symbol_kind,
        member_name,
        descriptor,
        assessment: "UNASSESSED",
        notes: "Classification requires compatibility evidence from a later phase.",
        tests: Vec::new(),
    }
}

fn required_path(args: &[String], name: &str) -> Result<PathBuf> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| PathBuf::from(&pair[1]))
        .ok_or_else(|| format!("missing required option {name}").into())
}

fn inventory_jar(path: &Path) -> Result<(Vec<ClassContract>, Vec<Resource>, usize, usize)> {
    let mut archive = ZipArchive::new(File::open(path)?)?;
    let jar_entries = archive.len();
    let mut classes = Vec::new();
    let mut resources = Vec::new();
    let mut class_entries = 0;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }
        let path = entry.name().to_owned();
        let size = entry.size();
        let compressed_size = entry.compressed_size();
        let crc32 = entry.crc32();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        if Path::new(&path)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("class"))
        {
            class_entries += 1;
            let class_file = ClassFile::from_slice(&bytes)
                .map_err(|error| format!("failed to parse {path}: {error}"))?;
            if let Some(class) = class_contract(&class_file)? {
                classes.push(class);
            }
        } else {
            let kind = resource_kind(&path);
            let utf8_content = matches!(kind, "manifest" | "service-provider")
                .then(|| String::from_utf8(bytes.clone()).ok())
                .flatten();
            resources.push(Resource {
                path,
                kind,
                size,
                compressed_size,
                crc32,
                sha256: sha256_bytes(&bytes),
                utf8_content,
            });
        }
    }
    resources.sort_by(|left, right| left.path.cmp(&right.path));
    Ok((classes, resources, jar_entries, class_entries))
}

fn class_contract(class: &ClassFile<'_>) -> Result<Option<ClassContract>> {
    let visibility = class_visibility(class)?;
    if visibility == "package" || visibility == "private" {
        return Ok(None);
    }
    let pool = &class.constant_pool;
    let binary_name = dotted(&class.class_name()?.to_string());
    let superclass = (class.super_class != 0)
        .then(|| {
            pool.try_get_class(class.super_class)
                .map(ToString::to_string)
        })
        .transpose()?
        .map(|name| dotted(&name));
    let interfaces = class
        .interfaces
        .iter()
        .map(|index| pool.try_get_class(*index).map(ToString::to_string))
        .collect::<std::result::Result<Vec<_>, _>>()?
        .into_iter()
        .map(|name| dotted(&name))
        .collect();
    let fields = class
        .fields
        .iter()
        .filter(|field| {
            field
                .access_flags
                .intersects(FieldAccessFlags::PUBLIC | FieldAccessFlags::PROTECTED)
        })
        .map(|field| {
            Ok(FieldContract {
                name: pool.try_get_utf8(field.name_index)?.to_string(),
                descriptor: pool.try_get_utf8(field.descriptor_index)?.to_string(),
                access_flags: field.access_flags.bits(),
                access: field.access_flags.as_code(),
                generic_signature: signature(pool, &field.attributes)?,
                annotations: annotations(pool, &field.attributes)?,
                attributes: attribute_names(&field.attributes),
                constant_value: constant_value(pool, &field.attributes)?,
                enum_constant: field.access_flags.contains(FieldAccessFlags::ENUM),
                synthetic: field.access_flags.contains(FieldAccessFlags::SYNTHETIC),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let is_interface = class.access_flags.contains(ClassAccessFlags::INTERFACE);
    let methods = class
        .methods
        .iter()
        .filter(|method| {
            method
                .access_flags
                .intersects(MethodAccessFlags::PUBLIC | MethodAccessFlags::PROTECTED)
        })
        .map(|method| {
            let has_code = method
                .attributes
                .iter()
                .any(|attribute| matches!(attribute, Attribute::Code { .. }));
            Ok(MethodContract {
                name: pool.try_get_utf8(method.name_index)?.to_string(),
                descriptor: pool.try_get_utf8(method.descriptor_index)?.to_string(),
                access_flags: method.access_flags.bits(),
                access: method.access_flags.as_code(),
                generic_signature: signature(pool, &method.attributes)?,
                annotations: annotations(pool, &method.attributes)?,
                attributes: attribute_names(&method.attributes),
                checked_exceptions: checked_exceptions(pool, &method.attributes)?,
                annotation_default: annotation_default(&method.attributes),
                default_interface_method: is_interface
                    && has_code
                    && !method.access_flags.intersects(
                        MethodAccessFlags::ABSTRACT
                            | MethodAccessFlags::STATIC
                            | MethodAccessFlags::PRIVATE,
                    ),
                bridge: method.access_flags.contains(MethodAccessFlags::BRIDGE),
                synthetic: method.access_flags.contains(MethodAccessFlags::SYNTHETIC),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Some(ClassContract {
        binary_name,
        classfile_major: class.version.major(),
        classfile_minor: class.version.minor(),
        access_flags: class.access_flags.bits(),
        access: class.access_flags.as_code(),
        visibility,
        superclass,
        interfaces,
        generic_signature: signature(pool, &class.attributes)?,
        annotations: annotations(pool, &class.attributes)?,
        attributes: attribute_names(&class.attributes),
        inner_classes: inner_classes(class)?,
        fields,
        methods,
    }))
}

fn class_visibility(class: &ClassFile<'_>) -> Result<&'static str> {
    let own_name = class.class_name()?;
    for attribute in &class.attributes {
        if let Attribute::InnerClasses { classes, .. } = attribute {
            for nested in classes {
                if class.constant_pool.try_get_class(nested.class_info_index)? == own_name {
                    return Ok(if nested.access_flags.bits() & 0x0001 != 0 {
                        "public"
                    } else if nested.access_flags.bits() & 0x0004 != 0 {
                        "protected"
                    } else if nested.access_flags.bits() & 0x0002 != 0 {
                        "private"
                    } else {
                        "package"
                    });
                }
            }
        }
    }
    Ok(if class.access_flags.contains(ClassAccessFlags::PUBLIC) {
        "public"
    } else {
        "package"
    })
}

fn inner_classes(class: &ClassFile<'_>) -> Result<Vec<InnerClassContract>> {
    let mut result = Vec::new();
    for attribute in &class.attributes {
        if let Attribute::InnerClasses { classes, .. } = attribute {
            for nested in classes {
                result.push(inner_class(class, nested)?);
            }
        }
    }
    result.sort_by(|left, right| left.binary_name.cmp(&right.binary_name));
    Ok(result)
}

fn inner_class(class: &ClassFile<'_>, nested: &InnerClass) -> Result<InnerClassContract> {
    let pool = &class.constant_pool;
    Ok(InnerClassContract {
        binary_name: dotted(&pool.try_get_class(nested.class_info_index)?.to_string()),
        outer_binary_name: (nested.outer_class_info_index != 0)
            .then(|| {
                pool.try_get_class(nested.outer_class_info_index)
                    .map(ToString::to_string)
            })
            .transpose()?
            .map(|name| dotted(&name)),
        inner_name: (nested.name_index != 0)
            .then(|| {
                pool.try_get_utf8(nested.name_index)
                    .map(ToString::to_string)
            })
            .transpose()?,
        access_flags: nested.access_flags.bits(),
        access: nested.access_flags.to_string(),
    })
}

fn signature(
    pool: &ristretto_classfile::ConstantPool<'_>,
    attributes: &[Attribute],
) -> Result<Option<String>> {
    attributes
        .iter()
        .find_map(|attribute| match attribute {
            Attribute::Signature {
                signature_index, ..
            } => Some(pool.try_get_utf8(*signature_index).map(ToString::to_string)),
            _ => None,
        })
        .transpose()
        .map_err(Into::into)
}

fn attribute_names(attributes: &[Attribute]) -> Vec<String> {
    attributes
        .iter()
        .map(|attribute| attribute.name().to_owned())
        .collect()
}

fn annotations(
    pool: &ristretto_classfile::ConstantPool<'_>,
    attributes: &[Attribute],
) -> Result<Vec<AnnotationContract>> {
    let mut result = Vec::new();
    for attribute in attributes {
        match attribute {
            Attribute::RuntimeVisibleAnnotations { annotations, .. } => {
                add_annotations(pool, annotations, true, "declaration", &mut result)?;
            }
            Attribute::RuntimeInvisibleAnnotations { annotations, .. } => {
                add_annotations(pool, annotations, false, "declaration", &mut result)?;
            }
            Attribute::RuntimeVisibleParameterAnnotations {
                parameter_annotations,
                ..
            } => {
                for parameter in parameter_annotations {
                    add_annotations(pool, &parameter.annotations, true, "parameter", &mut result)?;
                }
            }
            Attribute::RuntimeInvisibleParameterAnnotations {
                parameter_annotations,
                ..
            } => {
                for parameter in parameter_annotations {
                    add_annotations(
                        pool,
                        &parameter.annotations,
                        false,
                        "parameter",
                        &mut result,
                    )?;
                }
            }
            Attribute::RuntimeVisibleTypeAnnotations {
                type_annotations, ..
            } => add_type_annotations(pool, type_annotations, true, &mut result)?,
            Attribute::RuntimeInvisibleTypeAnnotations {
                type_annotations, ..
            } => add_type_annotations(pool, type_annotations, false, &mut result)?,
            _ => {}
        }
    }
    Ok(result)
}

fn add_annotations(
    pool: &ristretto_classfile::ConstantPool<'_>,
    annotations: &[Annotation],
    visible: bool,
    target: &'static str,
    result: &mut Vec<AnnotationContract>,
) -> Result<()> {
    for annotation in annotations {
        result.push(AnnotationContract {
            descriptor: pool.try_get_utf8(annotation.type_index)?.to_string(),
            visible,
            target,
            values: format!("{:?}", annotation.elements),
        });
    }
    Ok(())
}

fn add_type_annotations(
    pool: &ristretto_classfile::ConstantPool<'_>,
    annotations: &[TypeAnnotation],
    visible: bool,
    result: &mut Vec<AnnotationContract>,
) -> Result<()> {
    for annotation in annotations {
        result.push(AnnotationContract {
            descriptor: pool.try_get_utf8(annotation.type_index)?.to_string(),
            visible,
            target: "type",
            values: format!(
                "target={:?}; path={:?}; elements={:?}",
                annotation.target_type, annotation.type_path, annotation.elements
            ),
        });
    }
    Ok(())
}

fn checked_exceptions(
    pool: &ristretto_classfile::ConstantPool<'_>,
    attributes: &[Attribute],
) -> Result<Vec<String>> {
    let mut result = Vec::new();
    for attribute in attributes {
        if let Attribute::Exceptions {
            exception_indexes, ..
        } = attribute
        {
            for index in exception_indexes {
                result.push(dotted(&pool.try_get_class(*index)?.to_string()));
            }
        }
    }
    Ok(result)
}

fn annotation_default(attributes: &[Attribute]) -> Option<String> {
    attributes.iter().find_map(|attribute| match attribute {
        Attribute::AnnotationDefault { element, .. } => Some(format!("{element:?}")),
        _ => None,
    })
}

fn constant_value(
    pool: &ristretto_classfile::ConstantPool<'_>,
    attributes: &[Attribute],
) -> Result<Option<String>> {
    let Some(index) = attributes.iter().find_map(|attribute| match attribute {
        Attribute::ConstantValue {
            constant_value_index,
            ..
        } => Some(*constant_value_index),
        _ => None,
    }) else {
        return Ok(None);
    };
    let value = match pool.try_get(index)? {
        Constant::Integer(value) => value.to_string(),
        Constant::Float(value) => format!("{value:?}"),
        Constant::Long(value) => value.to_string(),
        Constant::Double(value) => format!("{value:?}"),
        Constant::String(_) => format!("{:?}", pool.try_get_string(index)?.to_string()),
        constant => format!("{constant:?}"),
    };
    Ok(Some(value))
}

fn external_types(classes: &[ClassContract]) -> Vec<String> {
    let mut types = BTreeSet::new();
    for class in classes {
        if let Some(superclass) = &class.superclass {
            maybe_external(superclass, &mut types);
        }
        for interface in &class.interfaces {
            maybe_external(interface, &mut types);
        }
        collect_types(class.generic_signature.as_deref(), &mut types);
        collect_annotation_types(&class.annotations, &mut types);
        for field in &class.fields {
            collect_types(Some(&field.descriptor), &mut types);
            collect_types(field.generic_signature.as_deref(), &mut types);
            collect_annotation_types(&field.annotations, &mut types);
        }
        for method in &class.methods {
            collect_types(Some(&method.descriptor), &mut types);
            collect_types(method.generic_signature.as_deref(), &mut types);
            collect_annotation_types(&method.annotations, &mut types);
            for exception in &method.checked_exceptions {
                maybe_external(exception, &mut types);
            }
        }
    }
    types.into_iter().collect()
}

fn collect_annotation_types(annotations: &[AnnotationContract], types: &mut BTreeSet<String>) {
    for annotation in annotations {
        collect_types(Some(&annotation.descriptor), types);
    }
}

fn collect_types(text: Option<&str>, types: &mut BTreeSet<String>) {
    let Some(text) = text else { return };
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'L' {
            let start = index + 1;
            let mut end = start;
            while end < bytes.len() && bytes[end] != b';' && bytes[end] != b'<' {
                end += 1;
            }
            if end > start {
                maybe_external(&text[start..end], types);
            }
            index = end;
        }
        index += 1;
    }
}

fn maybe_external(name: &str, types: &mut BTreeSet<String>) {
    let slashed = name.replace('.', "/");
    let is_jdk = [
        "java/",
        "javax/",
        "jdk/",
        "sun/",
        "org/w3c/dom/",
        "org/xml/sax/",
    ]
    .iter()
    .any(|prefix| slashed.starts_with(prefix));
    if !is_jdk && !slashed.starts_with(LAVAPLAYER_PREFIX) {
        types.insert(dotted(&slashed));
    }
}

fn resource_kind(path: &str) -> &'static str {
    if path.eq_ignore_ascii_case("META-INF/MANIFEST.MF") {
        "manifest"
    } else if path.starts_with("META-INF/services/") {
        "service-provider"
    } else if path.starts_with("META-INF/") {
        "meta-inf"
    } else {
        "resource"
    }
}

fn parse_pom_dependencies(pom: &str) -> Result<Vec<PomDependency>> {
    let dependencies =
        element_body(pom, "dependencies").ok_or("POM has no dependencies element")?;
    let mut result = Vec::new();
    let mut remainder = dependencies;
    while let Some(start) = remainder.find("<dependency>") {
        remainder = &remainder[start + "<dependency>".len()..];
        let end = remainder
            .find("</dependency>")
            .ok_or("unclosed POM dependency")?;
        let dependency = &remainder[..end];
        result.push(PomDependency {
            group_id: required_element(dependency, "groupId")?,
            artifact_id: required_element(dependency, "artifactId")?,
            version: required_element(dependency, "version")?,
            scope: element_body(dependency, "scope")
                .unwrap_or("compile")
                .trim()
                .to_owned(),
            optional: element_body(dependency, "optional").map(|value| value.trim().to_owned()),
            classifier: element_body(dependency, "classifier").map(|value| value.trim().to_owned()),
            dependency_type: element_body(dependency, "type").map(|value| value.trim().to_owned()),
        });
        remainder = &remainder[end + "</dependency>".len()..];
    }
    Ok(result)
}

fn required_element(text: &str, name: &str) -> Result<String> {
    element_body(text, name)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("POM dependency is missing {name}").into())
}

fn element_body<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let start = text.find(&open)? + open.len();
    let end = text[start..].find(&close)? + start;
    Some(&text[start..end])
}

fn inventory_built_in_sources(path: &Path) -> Result<BuiltInSources> {
    let mut archive = ZipArchive::new(File::open(path)?)?;
    let mut entry = archive.by_name(SOURCE_MANAGERS_PATH)?;
    let mut source = String::new();
    entry.read_to_string(&mut source)?;
    let remote_body = method_body(
        &source,
        "registerRemoteSources(AudioPlayerManager playerManager, MediaContainerRegistry containerRegistry)",
    )?;
    let local_body = method_body(
        &source,
        "registerLocalSource(AudioPlayerManager playerManager, MediaContainerRegistry containerRegistry)",
    )?;
    Ok(BuiltInSources {
        evidence_source: SOURCE_MANAGERS_PATH,
        remote_registration_order: registered_managers(remote_body),
        local_registration_order: registered_managers(local_body),
    })
}

fn method_body<'a>(source: &'a str, signature: &str) -> Result<&'a str> {
    let signature_start = source
        .find(signature)
        .ok_or_else(|| format!("source method not found: {signature}"))?;
    let open = source[signature_start..]
        .find('{')
        .map(|offset| signature_start + offset)
        .ok_or("source method has no body")?;
    let mut depth = 0_u32;
    for (offset, byte) in source.as_bytes()[open..].iter().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Ok(&source[open + 1..open + offset]);
                }
            }
            _ => {}
        }
    }
    Err("unclosed source method body".into())
}

fn registered_managers(body: &str) -> Vec<String> {
    body.lines()
        .filter_map(|line| {
            let call = line
                .trim()
                .strip_prefix("playerManager.registerSourceManager(")?;
            let expression = call.strip_suffix(");")?;
            let class = expression
                .strip_prefix("new ")
                .unwrap_or(expression)
                .split(['(', '.'])
                .next()?;
            class
                .ends_with("AudioSourceManager")
                .then(|| class.to_owned())
        })
        .collect()
}

fn validate_built_in_sources(sources: &BuiltInSources, classes: &[ClassContract]) -> Result<()> {
    if sources.remote_registration_order.is_empty() || sources.local_registration_order.is_empty() {
        return Err("source registration extraction yielded an empty order".into());
    }
    for manager in sources
        .remote_registration_order
        .iter()
        .chain(&sources.local_registration_order)
    {
        if !classes
            .iter()
            .any(|class| class.binary_name.ends_with(&format!(".{manager}")))
        {
            return Err(format!("registered source manager is absent from JAR: {manager}").into());
        }
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String> {
    Ok(sha256_bytes(&fs::read(path)?))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut result = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(result, "{byte:02x}").expect("writing to a String cannot fail");
    }
    result
}

fn dotted(name: &str) -> String {
    name.replace('/', ".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_external_types_from_descriptors_and_signatures() {
        let mut types = BTreeSet::new();
        collect_types(
            Some("(Ljava/lang/String;Lorg/slf4j/Logger;)Lcom/example/Thing;"),
            &mut types,
        );
        assert_eq!(
            types.into_iter().collect::<Vec<_>>(),
            ["com.example.Thing", "org.slf4j.Logger"]
        );
    }

    #[test]
    fn parses_pom_dependency_defaults() -> Result<()> {
        let dependencies = parse_pom_dependencies(
            "<project><dependencies><dependency><groupId>a</groupId><artifactId>b</artifactId><version>1</version></dependency></dependencies></project>",
        )?;
        assert_eq!(dependencies.len(), 1);
        assert_eq!(dependencies[0].scope, "compile");
        Ok(())
    }

    #[test]
    fn extracts_registration_order() {
        let body = "playerManager.registerSourceManager(new FirstAudioSourceManager());\nplayerManager.registerSourceManager(SecondAudioSourceManager.createDefault());";
        assert_eq!(
            registered_managers(body),
            ["FirstAudioSourceManager", "SecondAudioSourceManager"]
        );
    }

    #[test]
    fn extracts_balanced_method_body() -> Result<()> {
        let source = "void example() { if (true) { work(); } } void next() {}";
        assert_eq!(
            method_body(source, "example()")?.trim(),
            "if (true) { work(); }"
        );
        Ok(())
    }
}
