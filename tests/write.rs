mod common;

use std::path::PathBuf;

use hao::{
    dotnet::il::{Instruction, OpCode, Operand},
    Module,
};


fn out_dir(sample: &common::Sample, name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::create_dir_all(&dir).unwrap();
    for config in ["Sample.runtimeconfig.json", "Sample.exe.config"] {
        if sample.dir.join(config).exists() {
            std::fs::copy(sample.dir.join(config), dir.join(config)).unwrap();
        }
    }
    dir
}

fn run(dir: &std::path::Path) -> String {
    common::sample().unwrap().run(dir).unwrap()
}

#[test]
fn rewrite_unmodified() {
    let sample = sample!();
    let module = Module::from_bytes(&sample.bytes).unwrap();
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

    let dir = out_dir(sample, "unmodified");
    std::fs::write(dir.join("Sample.dll"), &written).unwrap();
    assert_eq!(run(&dir), sample.expected_output);
}

#[test]
fn patch_instructions() {
    let sample = sample!();
    let module = Module::from_bytes(&sample.bytes).unwrap();
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
    let dir = out_dir(sample, "patched");
    std::fs::write(dir.join("Sample.dll"), &written).unwrap();
    let expected = sample
        .expected_output
        .replace("Hello from hao", "Patched by hao")
        .replace("add=42", "add=80");
    assert_eq!(run(&dir), expected);
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

#[test]
fn import_helpers() {
    use hao::dotnet::signature::{MethodSig, TypeSig};

    let sample = sample!();
    let mut module = Module::from_bytes(&sample.bytes).unwrap();
    let refs = module.member_refs.len();

    let write_line = module
        .import_method("System.Console", "WriteLine", MethodSig::new_static(TypeSig::VOID, vec![TypeSig::STRING]))
        .unwrap();
    assert_eq!(module.member_refs.len(), refs, "existing reference should be reused");

    let write = module
        .import_method("System.Console", "Write", MethodSig::new_static(TypeSig::VOID, vec![TypeSig::STRING]))
        .unwrap();
    assert_eq!(module.member_refs.len(), refs + 1);

    let local = module
        .import_method("Sample.Calc", "Add", MethodSig::new_static(TypeSig::INT32, vec![TypeSig::INT32, TypeSig::INT32]))
        .unwrap();
    assert!(matches!(local, hao::dotnet::il::MemberToken::Method(_)));

    let main = module.entry_point.clone().unwrap();
    main.edit_body(|body| {
        body.insert(
            0,
            [
                Instruction::ldstr("imported "),
                Instruction::call(write),
                Instruction::ldc_i4(1000),
                Instruction::ldc_i4(2),
                Instruction::call(local),
                Instruction::pop(),
                Instruction::ldstr("ok"),
                Instruction::call(write_line),
            ],
        );
    });

    let dir = out_dir(sample, "imported");
    module.write_to_path(dir.join("Sample.dll")).unwrap();
    assert_eq!(run(&dir), format!("imported ok\n{}", sample.expected_output));
}

#[test]
fn shortest_instruction_forms() {
    assert_eq!(Instruction::ldc_i4(-1).opcode, OpCode::Ldc_I4_M1);
    assert_eq!(Instruction::ldc_i4(100).opcode, OpCode::Ldc_I4_S);
    assert_eq!(Instruction::ldc_i4(1000).opcode, OpCode::Ldc_I4);
    assert_eq!(Instruction::ldarg(2).opcode, OpCode::Ldarg_2);
    assert_eq!(Instruction::ldloc(300).opcode, OpCode::Ldloc);
}

/// Adds a reference to an assembly Sample doesn't use and calls generic code in it.
#[test]
fn reference_new_assembly() {
    use hao::dotnet::{entries::Version, signature::{MethodSig, TypeSig}};

    let sample = sample!();
    let mut module = Module::from_bytes(&sample.bytes).unwrap();
    assert!(module.find_assembly_ref("System.Linq").is_none());

    let version = Version { major: 9, minor: 0, build: 0, revision: 0 };
    let token = [0xb0, 0x3f, 0x5f, 0x7f, 0x11, 0xd5, 0x0a, 0x3a];
    module.add_assembly_ref("System.Linq", version, &token);
    let again = module.add_assembly_ref("System.Linq", version, &token);
    assert_eq!(module.assembly_refs.iter().filter(|a| a.ptr_eq(&again)).count(), 1);

    let enumerable = module.import_type_from("System.Linq", "System.Linq.Enumerable").unwrap();
    let ienumerable = module
        .import_type_from("System.Runtime", "System.Collections.Generic.IEnumerable`1")
        .unwrap();
    let ints = TypeSig::generic_class(ienumerable, vec![TypeSig::INT32]);

    let range = module
        .import_method_on(&enumerable, "Range", MethodSig::new_static(ints.clone(), vec![TypeSig::INT32, TypeSig::INT32]))
        .unwrap();
    let sum = module
        .import_method_on(&enumerable, "Sum", MethodSig::new_static(TypeSig::INT32, vec![ints]))
        .unwrap();
    let write_line = module
        .import_method("System.Console", "WriteLine", MethodSig::new_static(TypeSig::VOID, vec![TypeSig::INT32]))
        .unwrap();

    module.entry_point.clone().unwrap().edit_body(|body| {
        body.insert(
            0,
            [
                Instruction::ldc_i4(1),
                Instruction::ldc_i4(3),
                Instruction::call(range),
                Instruction::call(sum),
                Instruction::call(write_line),
            ],
        );
    });

    let dir = out_dir(sample, "new_assembly_ref");
    module.write_to_path(dir.join("Sample.dll")).unwrap();
    assert_eq!(run(&dir), format!("6\n{}", sample.expected_output));
}

#[test]
fn reference_from_manifest() {
    use hao::dotnet::md::streams::tables_stream::AssemblyFlags;

    let Some(dir) = hao::resolver::PathAssemblyResolver::find_framework_dir() else {
        return;
    };
    let loader = Module::from_path_no_resolve(dir.join("System.Linq.dll")).unwrap();
    let sample = sample!();
    let mut module = Module::from_bytes(&sample.bytes).unwrap();

    let reference = module.add_reference_to(&loader).unwrap();
    {
        let r = reference.get();
        let manifest = loader.assembly.as_ref().unwrap().get();
        assert_eq!(r.name, "System.Linq");
        assert_eq!(r.version, manifest.version);
        assert!(r.flags.contains(AssemblyFlags::PublicKey));
        assert_eq!(r.public_key_or_token, manifest.public_key);
    }
    assert!(module.add_reference_to(&loader).unwrap().ptr_eq(&reference));

    let written = Module::from_bytes(&module.write().unwrap()).unwrap();
    let r = written.find_assembly_ref("System.Linq").unwrap();
    assert!(r.get().flags.contains(AssemblyFlags::PublicKey));
}

/// The same checks against the .NET Framework build, which references `mscorlib`
/// like Unity's Mono games. Runs it with Mono when Mono is installed.
#[test]
fn rewrite_and_patch_for_mono() {
    let sample = sample!(framework_sample);
    let module = Module::from_bytes(&sample.bytes).unwrap();
    assert!(module.find_assembly_ref("mscorlib").is_some());

    let reloaded = Module::from_bytes(&module.write().unwrap()).unwrap();
    assert_eq!(reloaded.all_methods().len(), module.all_methods().len());

    let add = module.find_method("Sample.Calc", "Add").unwrap();
    add.edit_body(|body| {
        let index = body.position(|i| i.opcode == OpCode::Add).unwrap();
        body.instructions[index] = Instruction::simple(OpCode::Mul);
    });

    let dir = out_dir(sample, "mono_patched");
    module.write_to_path(dir.join(sample.file_name)).unwrap();
    if let Some(output) = sample.run(&dir) {
        assert_eq!(output, sample.expected_output.replace("add=42", "add=80"));
    } else {
        eprintln!("mono not installed, only checked the rewritten structure");
    }
}
