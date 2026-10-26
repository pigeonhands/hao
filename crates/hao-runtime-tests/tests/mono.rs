#![cfg(hao_test_lib)]

mod common;

#[test]
fn mono_bindings() {
    let runtime = match hao_runtime::Runtime::mono(&hao_runtime::MonoOptions::default()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("skipping: {}", e);
            return;
        }
    };
    common::exercise(&runtime);
}
