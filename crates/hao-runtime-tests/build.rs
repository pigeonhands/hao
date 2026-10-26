//! Builds `dotnet/TestLib` with the dotnet SDK. Without the SDK the tests are skipped.

use std::{path::PathBuf, process::Command};

fn main() {
    println!("cargo::rustc-check-cfg=cfg(hao_test_lib)");
    println!("cargo::rerun-if-changed=dotnet");

    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("TestLib");
    let project = manifest.join("dotnet/TestLib/TestLib.csproj");
    let built = Command::new("dotnet")
        .args(["build", "-c", "Release", "-o"])
        .arg(&out)
        .arg(format!("-p:BaseIntermediateOutputPath={}/obj/", out.display()))
        .arg(&project)
        .output();
    match built {
        Ok(output) if output.status.success() => {
            println!("cargo::rustc-env=HAO_TEST_LIB={}", out.join("TestLib.dll").display());
            println!("cargo::rustc-cfg=hao_test_lib");
        }
        Ok(output) => println!(
            "cargo::warning=building TestLib failed, skipping binding tests: {}",
            String::from_utf8_lossy(&output.stdout).lines().filter(|l| l.contains("error")).collect::<Vec<_>>().join(" ")
        ),
        Err(_) => println!("cargo::warning=dotnet not found, skipping binding tests"),
    }
}
