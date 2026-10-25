//! Patches the IL of the sample assembly and writes a new assembly.
//!
//! ```sh
//! tests/fixtures/build.sh
//! cargo run --example patch_method -- tests/fixtures/out/Sample.dll out/Sample.dll
//! cp tests/fixtures/out/Sample.runtimeconfig.json out/
//! dotnet out/Sample.dll
//! ```

use hao::{
    dotnet::{
        csharp::WriteCSharp,
        il::{Instruction, OpCode},
        signature::{MethodSig, TypeSig},
    },
    Module,
};

fn main() -> hao::error::Result<()> {
    let mut args = std::env::args().skip(1);
    let (input, output) = match (args.next(), args.next()) {
        (Some(i), Some(o)) => (i, o),
        _ => panic!("usage: patch_method <Sample.dll> <output.dll>"),
    };
    let mut module = Module::from_path_no_resolve(&input)?;

    let write_line = module.import_method(
        "System.Console",
        "WriteLine",
        MethodSig::new_static(TypeSig::VOID, vec![TypeSig::STRING]),
    )?;

    let add = module.find_method("Sample.Calc", "Add").expect("Calc.Add");
    println!("before:\n{}", add.get().body.as_ref().unwrap().csharp());

    add.edit_body(|body| {
        // a + b  ->  a * b
        let index = body.position(|i| i.opcode == OpCode::Add).unwrap();
        body.instructions[index] = Instruction::simple(OpCode::Mul);

        // Console.WriteLine("Add called") at the start of the method.
        body.insert(0, [Instruction::ldstr("Add called"), Instruction::call(write_line)]);
    });
    println!("after:\n{}", add.get().body.as_ref().unwrap().csharp());

    let main = module.entry_point.clone().expect("entry point");
    main.edit_body(|body| {
        body.replace_operand(&"Hello from hao".into(), "Hello from a patched assembly".into())
    });

    if let Some(dir) = std::path::Path::new(&output).parent() {
        std::fs::create_dir_all(dir).ok();
    }
    module.write_to_path(&output)?;
    println!("wrote {}", output);
    Ok(())
}
