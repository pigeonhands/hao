#![cfg(feature = "reflect")]
//! Confirms the `reflect` feature derives `facet::Facet` across every category
//! of reflected type, that the entry handles reflect transparently, and that
//! the bitflags escape hatch still yields flag names.

use bitflags::Flags;
use facet::Facet;
use hao::{
    dotnet::{
        entries::{
            Entry, Field, Method, ModuleDef, TypeDef, TypeDefOrRef, WeakEntry,
        },
        il::{Instruction, OpCode, Operand},
        md::streams::tables_stream::TypeAttributes,
        signature::{ElementType, MethodSig, PrimitiveType, SignatureFlags, TypeSig},
    },
    Module,
};

fn assert_facet<'a, T: Facet<'a>>() {}

#[test]
fn every_category_is_reflected() {
    // The entry-point type.
    assert_facet::<Module>();
    // Struct entries behind shared handles.
    assert_facet::<TypeDef>();
    assert_facet::<Method>();
    assert_facet::<Field>();
    // The handle itself, generic over the entry (reflects through Arc/RwLock).
    assert_facet::<Entry<TypeDef>>();
    assert_facet::<WeakEntry<TypeDef>>();
    // A struct holding `uuid::Uuid` fields (facet's uuid feature).
    assert_facet::<ModuleDef>();
    // Data-carrying enums.
    assert_facet::<TypeDefOrRef>();
    assert_facet::<TypeSig>();
    assert_facet::<Operand>();
    // `#[repr(u8)]` enums shared with strum's FromRepr.
    assert_facet::<PrimitiveType>();
    assert_facet::<ElementType>();
    // Signatures and IL.
    assert_facet::<MethodSig>();
    assert_facet::<Instruction>();
    assert_facet::<OpCode>();
    // Bitflags, registered opaque.
    assert_facet::<TypeAttributes>();
    assert_facet::<SignatureFlags>();
}

#[test]
fn roots_are_exposed() {
    let roots = hao::reflect::roots();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].type_identifier, "Module");
}

#[test]
fn bitflags_names_reachable_via_flags_trait() {
    // Opaque in facet, but names come from bitflags' own Flags trait.
    let names: Vec<&str> = TypeAttributes::FLAGS.iter().map(|f| f.name()).collect();
    assert!(names.contains(&"Public"));
}
