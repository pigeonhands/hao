//! A small "game" assembly built with hao, standing in for one read from disk.

// Each example uses only some of this.
#![allow(dead_code)]

use hao::{
    dotnet::{
        entries::{
            CustomAttribute, CustomAttributeCtor, CustomAttributeValue, Entry, Method, TypeDef,
        },
        il::{Instruction, MemberToken, MethodBody, OpCode},
        md::streams::tables_stream::{MethodFlags, TypeAttributes},
        module::{ModuleKind, TargetRuntime},
        signature::{MethodSig, TypeSig},
    },
    Module,
};

/// Adds `[UnmanagedCallersOnly]`, which CoreCLR needs to call a method from native code.
pub fn unmanaged_callers_only(module: &mut Module, method: &Entry<Method>) {
    let attribute = module
        .corlib_type("System.Runtime.InteropServices.UnmanagedCallersOnlyAttribute")
        .unwrap();
    let MemberToken::MemberRef(ctor) = module
        .import_method_on(
            &attribute,
            ".ctor",
            MethodSig::new_instance(TypeSig::VOID, vec![]),
        )
        .unwrap()
    else {
        unreachable!()
    };
    method.get_mut().custom_attributes.push(
        CustomAttribute::new(
            CustomAttributeCtor::MemberRef(ctor),
            &CustomAttributeValue::default(),
        )
        .unwrap(),
    );
}

pub fn static_method(
    ty: &Entry<TypeDef>,
    name: &str,
    params: usize,
    instructions: Vec<Instruction>,
) -> Entry<Method> {
    let method = TypeDef::add_method(
        ty,
        Method::new(
            MethodFlags::Public | MethodFlags::Static | MethodFlags::HideBySig,
            name,
            MethodSig::new_static(TypeSig::INT32, vec![TypeSig::INT32; params]),
        ),
    );
    method.get_mut().body = Some(MethodBody {
        instructions,
        ..Default::default()
    });
    method
}

/// ```csharp
/// public static class Game.Player {
///     public static int TakeDamage(int health, int amount) => health - amount;
///     public static int Score(int kills) => kills * 100;
///
///     // Entry points for native code.
///     [UnmanagedCallersOnly] public static int Hit(int health, int amount) => TakeDamage(health, amount);
///     [UnmanagedCallersOnly] public static int GetScore(int kills) => Score(kills);
/// }
/// ```
pub fn game_assembly() -> Vec<u8> {
    let mut module = Module::new(
        "Game",
        ModuleKind::Dll,
        TargetRuntime::NetCoreImplementation(8),
    );
    let object = module.corlib_type("System.Object").unwrap();
    let player = module.add_type(TypeDef::new(
        TypeAttributes::Public | TypeAttributes::Abstract | TypeAttributes::Sealed,
        "Game",
        "Player",
        Some(object),
    ));
    let take_damage = static_method(
        &player,
        "TakeDamage",
        2,
        vec![
            Instruction::ldarg(0),
            Instruction::ldarg(1),
            Instruction::simple(OpCode::Sub),
            Instruction::ret(),
        ],
    );
    let score = static_method(
        &player,
        "Score",
        1,
        vec![
            Instruction::ldarg(0),
            Instruction::ldc_i4(100),
            Instruction::simple(OpCode::Mul),
            Instruction::ret(),
        ],
    );
    let hit = static_method(
        &player,
        "Hit",
        2,
        vec![
            Instruction::ldarg(0),
            Instruction::ldarg(1),
            Instruction::call(take_damage),
            Instruction::ret(),
        ],
    );
    let get_score = static_method(
        &player,
        "GetScore",
        1,
        vec![
            Instruction::ldarg(0),
            Instruction::call(score),
            Instruction::ret(),
        ],
    );
    unmanaged_callers_only(&mut module, &hit);
    unmanaged_callers_only(&mut module, &get_score);
    module.write().unwrap()
}

/// Renames a module's assembly, so an edited copy can be loaded next to the original.
pub fn rename(module: &Module, name: &str) {
    if let Some(assembly) = &module.assembly {
        assembly.get_mut().name = name.to_string();
    }
}
