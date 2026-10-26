#![cfg(hao_test_lib)]

mod common;

#[test]
fn coreclr_bindings() {
    if hao_runtime::find_dotnet_root().is_none() {
        eprintln!("skipping: .NET is not installed");
        return;
    }
    let runtime = hao_runtime::Runtime::coreclr().unwrap();
    common::exercise(&runtime);
}
