use std::{path::PathBuf, process::Command};

use hao::{
    dotnet::il::{Instruction, OpCode, Operand},
    Module,
};

const SAMPLE: &[u8] = include_bytes!("fixtures/Sample.dll");
const EXPECTED_OUTPUT: &str = include_str!("fixtures/expected_output.txt");

fn out_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/Sample.runtimeconfig.json"),
        dir.join("Sample.runtimeconfig.json"),
    )
    .unwrap();
    dir
}

/// Runs the assembly with `dotnet`, or returns `None` when no runtime is installed.
fn run(dir: &std::path::Path) -> Option<String> {
    let output = Command::new("dotnet").arg(dir.join("Sample.dll")).output().ok()?;
    assert!(
        output.status.success(),
        "dotnet failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Some(String::from_utf8(output.stdout).unwrap())
}

#[test]
fn rewrite_unmodified() {
    let module = Module::from_bytes(SAMPLE).unwrap();
    let written = module.write().unwrap();

    let reloaded = Module::from_bytes(&written).unwrap();
    assert_eq!(reloaded.all_types().len(), module.all_types().len());
    assert_eq!(reloaded.all_methods().len(), module.all_methods().len());
    for (a, b) in module.all_methods().iter().zip(reloaded.all_methods().iter()) {
        let (a, b) = (a.get(), b.get());
        assert_eq!(a.name, b.name);
        assert_eq!(
            a.body.as_ref().map(|b| b.instructions.len()),
            b.body.as_ref().map(|b| b.instructions.len()),
            "{}",
            a.name
        );
    }

    let dir = out_dir("unmodified");
    std::fs::write(dir.join("Sample.dll"), &written).unwrap();
    if let Some(output) = run(&dir) {
        assert_eq!(output, EXPECTED_OUTPUT);
    }
}

#[test]
fn patch_instructions() {
    let module = Module::from_bytes(SAMPLE).unwrap();
    let calc = module.find_type("Sample", "Calc").unwrap();

    let add = calc.get().find_method("Add").unwrap();
    {
        let mut add = add.get_mut();
        let body = add.body.as_mut().unwrap();
        let op = body.instructions.iter_mut().find(|i| i.opcode == OpCode::Add).unwrap();
        *op = Instruction::simple(OpCode::Mul);
    }

    let main = module.entry_point.clone().unwrap();
    {
        let mut main = main.get_mut();
        let body = main.body.as_mut().unwrap();
        for instruction in &mut body.instructions {
            if instruction.operand == Operand::String("Hello from hao".into()) {
                instruction.operand = Operand::String("Patched by hao".into());
            }
        }
    }

    let written = module.write().unwrap();
    let dir = out_dir("patched");
    std::fs::write(dir.join("Sample.dll"), &written).unwrap();
    if let Some(output) = run(&dir) {
        let expected = EXPECTED_OUTPUT
            .replace("Hello from hao", "Patched by hao")
            .replace("add=42", "add=80");
        assert_eq!(output, expected);
    }
}

fn signature(module: &Module) -> Vec<(String, usize, usize, usize)> {
    module
        .all_types()
        .iter()
        .map(|t| {
            let t = t.get();
            let instructions = t
                .methods
                .iter()
                .map(|m| m.get().body.as_ref().map_or(0, |b| b.instructions.len()))
                .sum();
            (t.full_name(), t.fields.len(), t.methods.len(), instructions)
        })
        .collect()
}

/// Rewrites every assembly in the installed shared framework and reloads it.
#[test]
#[ignore]
fn rewrite_shared_framework() {
    let dir = hao::resolver::PathAssemblyResolver::find_framework_dir().unwrap();
    let mut failures = Vec::new();
    let mut count = 0;
    for file in std::fs::read_dir(dir).unwrap() {
        let path = file.unwrap().path();
        if path.extension().is_none_or(|e| e != "dll") {
            continue;
        }
        let Ok(module) = Module::from_path_no_resolve(&path) else {
            continue;
        };
        let result = module
            .write()
            .and_then(|data| Module::from_bytes(&data))
            .map(|reloaded| signature(&module) == signature(&reloaded));
        match result {
            Ok(true) => count += 1,
            Ok(false) => failures.push(format!("{}: reloaded module differs", path.display())),
            Err(e) => failures.push(format!("{}: {}", path.display(), e)),
        }
    }
    assert!(failures.is_empty(), "{} ok, failures:\n{}", count, failures.join("\n"));
    println!("rewrote {} assemblies", count);
}

/// Framework assemblies are ReadyToRun images. The rewritten image must drop the
/// native code and be marked IL only again.
#[test]
fn rewrite_ready_to_run() {
    use pewter::pe::{coff::ImageFileMachine, sections::cor20::Cor20Flags, PEFile};

    let Some(dir) = hao::resolver::PathAssemblyResolver::find_framework_dir() else {
        return;
    };
    let module = Module::from_path_no_resolve(dir.join("System.Collections.Immutable.dll")).unwrap();
    let written = module.write().unwrap();

    let pe = PEFile::parse(&written).unwrap();
    let cor20 = pe.read_clr_runtime_header().unwrap().unwrap();
    assert!(cor20.managed_native_header.is_null());
    assert!(cor20.flags.contains(Cor20Flags::IL_ONLY));
    assert!(!cor20.flags.contains(Cor20Flags::IL_LIBRARY));
    assert!(!matches!(pe.coff_header.machine, ImageFileMachine::Other(_)));
    let dirs = &pe.optional_header.as_ref().unwrap().data_directories;
    assert!(dirs.exception_table.is_null());
    Module::from_bytes(&written).unwrap();
}
