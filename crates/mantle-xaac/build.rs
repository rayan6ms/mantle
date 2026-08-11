use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

const REVISION: &str = "8d9809f480fb56c68ff6b76927aceb382d55045e";

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let root = manifest.join("../..");
    let pinned_source = root.join("third_party/libxaac/source");
    let patch =
        root.join("third_party/libxaac/patches/0001-bound-decoder-indices-and-lifetimes.patch");
    println!("cargo:rerun-if-changed={}", patch.display());
    println!(
        "cargo:rerun-if-changed={}",
        root.join(".gitmodules").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        pinned_source.join("CMakeLists.txt").display()
    );

    assert!(
        pinned_source.join("CMakeLists.txt").is_file(),
        "libxaac submodule is absent; run `git submodule update --init third_party/libxaac/source`"
    );
    let actual_revision = output(
        Command::new("git")
            .arg("-C")
            .arg(&pinned_source)
            .args(["rev-parse", "HEAD"]),
    );
    assert_eq!(
        actual_revision, REVISION,
        "libxaac submodule revision mismatch"
    );

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("build output directory"));
    let source = out.join("libxaac-source");
    prepare_source(&pinned_source, &source, &patch);
    verify_source_tree(&source, &patch, &out.join("libxaac-verify-index"));

    let build = out.join("cmake-build");
    run(Command::new("cmake")
        .arg("-S")
        .arg(&source)
        .arg("-B")
        .arg(&build)
        .args([
            "-DCMAKE_BUILD_TYPE=Release",
            "-DCMAKE_COMPILE_WARNING_AS_ERROR=ON",
            "-DCMAKE_POSITION_INDEPENDENT_CODE=ON",
        ]));
    run(Command::new("cmake").arg("--build").arg(&build).args([
        "--config",
        "Release",
        "--target",
        "libxaacdec",
        "--parallel",
        "2",
    ]));

    let archive = find_archive(&build).unwrap_or_else(|| {
        panic!(
            "decoder build completed without a libxaacdec archive under {}",
            build.display()
        )
    });
    println!(
        "cargo:rustc-link-search=native={}",
        archive.parent().expect("archive parent").display()
    );
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-lib=static=libxaacdec");
    } else {
        println!("cargo:rustc-link-lib=static=xaacdec");
    }
    if env::var("CARGO_CFG_UNIX").is_ok() {
        println!("cargo:rustc-link-lib=m");
    }
}

fn prepare_source(pinned: &Path, source: &Path, patch: &Path) {
    let patch_present = source.join("CMakeLists.txt").is_file()
        && succeeds(
            Command::new("git")
                .arg("-C")
                .arg(source)
                .args(["apply", "--reverse", "--check"])
                .arg(patch),
        );
    if patch_present {
        return;
    }
    if source.exists() {
        fs::remove_dir_all(source).expect("remove stale libxaac build source");
    }
    run(Command::new("git")
        .args(["clone", "--quiet", "--no-checkout"])
        .arg(pinned)
        .arg(source));
    run(Command::new("git")
        .arg("-C")
        .arg(source)
        .args(["checkout", "--quiet", "--detach", REVISION]));
    run(Command::new("git")
        .arg("-C")
        .arg(source)
        .args(["apply", "--check"])
        .arg(patch));
    run(Command::new("git")
        .arg("-C")
        .arg(source)
        .arg("apply")
        .arg(patch));
    run(Command::new("git")
        .arg("-C")
        .arg(source)
        .args(["diff", "--check"]));
}

fn find_archive(directory: &Path) -> Option<PathBuf> {
    for entry in fs::read_dir(directory).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_archive(&path) {
                return Some(found);
            }
        } else if matches!(
            path.file_name().and_then(OsStr::to_str),
            Some("libxaacdec.a" | "libxaacdec.lib")
        ) {
            return Some(path);
        }
    }
    None
}

fn verify_source_tree(source: &Path, patch: &Path, index: &Path) {
    if index.exists() {
        fs::remove_file(index).expect("remove stale verification index");
    }
    run(Command::new("git")
        .arg("-C")
        .arg(source)
        .args(["read-tree", "HEAD"])
        .env("GIT_INDEX_FILE", index));
    run(Command::new("git")
        .arg("-C")
        .arg(source)
        .args(["apply", "--cached"])
        .arg(patch)
        .env("GIT_INDEX_FILE", index));
    let expected = output(
        Command::new("git")
            .arg("-C")
            .arg(source)
            .arg("write-tree")
            .env("GIT_INDEX_FILE", index),
    );
    run(Command::new("git")
        .arg("-C")
        .arg(source)
        .args(["add", "--update"])
        .env("GIT_INDEX_FILE", index));
    let actual = output(
        Command::new("git")
            .arg("-C")
            .arg(source)
            .arg("write-tree")
            .env("GIT_INDEX_FILE", index),
    );
    fs::remove_file(index).expect("remove verification index");
    assert_eq!(
        actual, expected,
        "libxaac source differs from Mantle's patch"
    );
}

fn output(command: &mut Command) -> String {
    let output = command.output().expect("run build command");
    assert!(
        output.status.success(),
        "command failed with {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("UTF-8 command output")
        .trim()
        .to_owned()
}

fn succeeds(command: &mut Command) -> bool {
    command.status().is_ok_and(|status| status.success())
}

fn run(command: &mut Command) -> ExitStatus {
    let status = command.status().expect("run build command");
    assert!(status.success(), "command failed with {status}");
    status
}
