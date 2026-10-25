//! Patches the IL of `tests/fixtures/Sample.dll` and writes a new assembly.
//!
//! ```sh
//! cargo run --example patch_method -- tests/fixtures/Sample.dll out/Sample.dll
//! cp tests/fixtures/Sample.runtimeconfig.json out/
//! dotnet out/Sample.dll
//! ```

use hao::{
    dotnet::{
        csharp::{write_il, CSharpWriter},
        entries::{Entry, MemberRef, MemberRefParent, ResolutionScope, TypeRef},
        il::{Instruction, MemberToken, OpCode, Operand},
        signature::{MemberRefSig, MethodSig, PrimitiveType, TypeSig},
    },
    Module,
};

fn print_il(title: &str, method: &Entry<hao::dotnet::entries::Method>) {
    let mut w = CSharpWriter::new(String::new());
    write_il(&mut w, method.get().body.as_ref().unwrap()).unwrap();
    println!("{}:\n{}", title, w.into_inner());
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (input, output) = match (args.next(), args.next()) {
        (Some(i), Some(o)) => (i, o),
        _ => panic!("usage: patch_method <Sample.dll> <output.dll>"),
    };
    let module = Module::from_path_no_resolve(&input).unwrap();

    // A reference to `void System.Console::WriteLine(string)`. The writer adds
    // new references to the metadata automatically.
    let console_asm = module
        .assembly_refs
        .iter()
        .find(|a| a.get().name == "System.Console")
        .expect("Sample references System.Console")
        .clone();
    let console = Entry::new(TypeRef::new(
        ResolutionScope::AssemblyRef(console_asm),
        "System",
        "Console",
    ));
    let write_line = Entry::new(MemberRef {
        parent: MemberRefParent::TypeRef(console),
        name: "WriteLine".into(),
        signature: MemberRefSig::Method(MethodSig::new_static(
            TypeSig::VOID,
            vec![TypeSig::Primitive(PrimitiveType::String)],
        )),
        custom_attributes: Vec::new(),
    });

    let calc = module.find_type("Sample", "Calc").expect("Sample.Calc");
    let add = calc.get().find_method("Add").expect("Calc.Add");
    print_il("Calc.Add before", &add);
    {
        let mut add = add.get_mut();
        let body = add.body.as_mut().unwrap();

        // a + b  ->  a * b
        let index = body.position(|i| i.opcode == OpCode::Add).unwrap();
        body.instructions[index] = Instruction::simple(OpCode::Mul);

        // Console.WriteLine("Add called") at the start of the method.
        body.insert(
            0,
            [
                Instruction::new(OpCode::Ldstr, Operand::String("Add called".into())),
                Instruction::new(OpCode::Call, Operand::Token(MemberToken::MemberRef(write_line))),
            ],
        );
    }
    print_il("Calc.Add after", &add);

    // Replace a string literal in Main.
    let main = module.entry_point.clone().expect("entry point");
    for instruction in &mut main.get_mut().body.as_mut().unwrap().instructions {
        if instruction.operand == Operand::String("Hello from hao".into()) {
            instruction.operand = Operand::String("Hello from a patched assembly".into());
        }
    }

    if let Some(dir) = std::path::Path::new(&output).parent() {
        std::fs::create_dir_all(dir).unwrap();
    }
    module.write_to_path(&output).unwrap();
    println!("wrote {}", output);
}
