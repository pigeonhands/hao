//! Builds this crate for `wasm32-unknown-unknown`, runs it with Node (`run.mjs`)
//! and compares the results with native hao. Skipped without Node or the wasm target.
//! Set `HAO_IL2CPP_GAME` to also run an Il2Cpp game through it.

use std::{path::PathBuf, process::Command};

fn result(output: &str, name: &str) -> Option<i64> {
    let start = output.find(&format!("\"{}\":", name))? + name.len() + 3;
    let end = output[start..].find([',', '}'])? + start;
    output[start..end].parse().ok()
}

#[test]
fn runs_in_wasm() {
    if Command::new("node").arg("--version").output().is_err() {
        eprintln!("skipping: node is not installed");
        return;
    }
    let Some(framework) = hao::resolver::PathAssemblyResolver::find_framework_dir() else {
        eprintln!("skipping: no .NET framework to test with");
        return;
    };
    let target_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("wasm");
    let build = Command::new(env!("CARGO"))
        .args(["build", "--release", "-p", "hao-wasm-tests", "--target", "wasm32-unknown-unknown"])
        .env("CARGO_TARGET_DIR", &target_dir)
        .output()
        .unwrap();
    if !build.status.success() {
        let stderr = String::from_utf8_lossy(&build.stderr);
        if stderr.contains("wasm32-unknown-unknown") && stderr.contains("not installed") {
            eprintln!("skipping: rustup target add wasm32-unknown-unknown");
            return;
        }
        panic!("wasm build failed:\n{}", stderr);
    }

    let assembly = framework.join("System.Linq.dll");
    let mut node = Command::new("node");
    node.arg(concat!(env!("CARGO_MANIFEST_DIR"), "/run.mjs"))
        .arg(target_dir.join("wasm32-unknown-unknown/release/hao_wasm_tests.wasm"))
        .arg(&assembly);
    let game = std::env::var_os("HAO_IL2CPP_GAME").map(|dir| hao_il2cpp::Game::open(&dir).map(|g| (dir, g)));
    if let Some(Ok((dir, _))) = &game {
        let metadata = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path().join("il2cpp_data/Metadata/global-metadata.dat")))
            .find(|p| p.exists())
            .unwrap();
        node.arg(metadata).arg(PathBuf::from(dir).join("GameAssembly.so"));
    }
    let output = node.output().unwrap();
    let output = String::from_utf8_lossy(&output.stdout).into_owned() + &String::from_utf8_lossy(&output.stderr);

    let native = hao::Module::from_path_no_resolve(&assembly).unwrap();
    assert_eq!(result(&output, "rewrite_assembly"), Some(native.all_types().len() as i64), "{}", output);
    assert_eq!(result(&output, "create_assembly"), Some(2), "{}", output);
    if let Some(game) = game {
        let (_, game) = game.unwrap();
        assert_eq!(result(&output, "il2cpp_assemblies"), Some(game.image_names().count() as i64), "{}", output);
    }
}
