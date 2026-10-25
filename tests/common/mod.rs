//! Builds `tests/fixtures/Sample` with the dotnet SDK. Tests that need it are
//! skipped when the SDK is not installed.

#![allow(dead_code)]

use std::{path::PathBuf, process::Command, sync::OnceLock};

pub struct Sample {
    pub dir: PathBuf,
    pub bytes: Vec<u8>,
    pub expected_output: String,
}

fn build() -> Option<Sample> {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sample");
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/Sample");
    let src_copy = root.join("src");
    let dir = root.join("bin");
    let dll = dir.join("Sample.dll");

    if !dll.exists() {
        std::fs::create_dir_all(&src_copy).ok()?;
        for entry in std::fs::read_dir(&source).ok()? {
            let path = entry.ok()?.path();
            if path.is_file() {
                std::fs::copy(&path, src_copy.join(path.file_name()?)).ok()?;
            }
        }
        let status = Command::new("dotnet")
            .args(["build", "-c", "Release", "-o"])
            .arg(&dir)
            .arg(src_copy.join("Sample.csproj"))
            .output()
            .ok()?;
        assert!(
            status.status.success(),
            "building the sample failed:\n{}",
            String::from_utf8_lossy(&status.stdout)
        );
    }

    let output = Command::new("dotnet").arg(&dll).output().ok()?;
    Some(Sample {
        bytes: std::fs::read(&dll).ok()?,
        expected_output: String::from_utf8(output.stdout).ok()?,
        dir,
    })
}

pub fn sample() -> Option<&'static Sample> {
    static SAMPLE: OnceLock<Option<Sample>> = OnceLock::new();
    let sample = SAMPLE.get_or_init(build).as_ref();
    if sample.is_none() {
        eprintln!("skipping: the dotnet SDK is needed to build the sample assembly");
    }
    sample
}

/// Returns the built sample, or returns from the test if it can't be built.
#[macro_export]
macro_rules! sample {
    () => {
        match common::sample() {
            Some(sample) => sample,
            None => return,
        }
    };
}
