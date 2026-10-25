//! Builds `tests/fixtures/Sample` with the dotnet SDK. Tests that need it are
//! skipped when the SDK is not installed.

#![allow(dead_code)]

use std::{path::PathBuf, process::Command, sync::OnceLock};

pub struct Sample {
    pub dir: PathBuf,
    pub file_name: &'static str,
    pub bytes: Vec<u8>,
    pub expected_output: String,
    /// Command that runs the assembly, `None` if the runtime isn't installed.
    pub runner: Option<&'static str>,
}

impl Sample {
    pub fn run(&self, dir: &std::path::Path) -> Option<String> {
        let output = Command::new(self.runner?).arg(dir.join(self.file_name)).output().ok()?;
        assert!(
            output.status.success(),
            "{} failed: {}",
            self.runner?,
            String::from_utf8_lossy(&output.stderr)
        );
        Some(String::from_utf8(output.stdout).unwrap())
    }
}

fn has_command(name: &str) -> bool {
    Command::new(name).arg("--version").output().is_ok()
}

fn build() -> Option<Sample> {
    build_target("sample", "net9.0", "Sample.dll", Some("dotnet"))
}

fn build_framework() -> Option<Sample> {
    let runner = has_command("mono").then_some("mono");
    build_target("sample-net472", "net472", "Sample.exe", runner)
}

fn build_target(
    name: &str,
    framework: &str,
    file_name: &'static str,
    runner: Option<&'static str>,
) -> Option<Sample> {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/Sample");
    let src_copy = root.join("src");
    let dir = root.join("bin");
    let dll = dir.join(file_name);

    if !dll.exists() {
        std::fs::create_dir_all(&src_copy).ok()?;
        for entry in std::fs::read_dir(&source).ok()? {
            let path = entry.ok()?.path();
            if path.is_file() {
                let contents = std::fs::read(&path).ok()?;
                let contents = if path.extension().is_some_and(|e| e == "csproj") {
                    String::from_utf8(contents)
                        .ok()?
                        .replace("<TargetFramework>net9.0</TargetFramework>", &format!("<TargetFramework>{}</TargetFramework>", framework))
                        .into_bytes()
                } else {
                    contents
                };
                std::fs::write(src_copy.join(path.file_name()?), contents).ok()?;
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

    let mut sample = Sample {
        bytes: std::fs::read(&dll).ok()?,
        expected_output: String::new(),
        file_name,
        runner,
        dir: dir.clone(),
    };
    if runner.is_some() {
        sample.expected_output = sample.run(&dir)?;
    }
    Some(sample)
}

/// The sample built for .NET Framework 4.7.2 (an `mscorlib` assembly like Unity's), run with Mono.
pub fn framework_sample() -> Option<&'static Sample> {
    static SAMPLE: OnceLock<Option<Sample>> = OnceLock::new();
    let sample = SAMPLE.get_or_init(build_framework).as_ref();
    if sample.is_none() {
        eprintln!("skipping: could not build the net472 sample");
    }
    sample
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
        sample!(sample)
    };
    ($which:ident) => {
        match common::$which() {
            Some(sample) => sample,
            None => return,
        }
    };
}
