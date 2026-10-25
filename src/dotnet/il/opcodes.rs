use strum::{EnumIter, FromRepr, IntoStaticStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperandType {
    None,
    ShortI,
    I,
    I8,
    ShortR,
    R,
    ShortBranch,
    Branch,
    Switch,
    ShortVar,
    Var,
    Method,
    Field,
    Type,
    Tok,
    String,
    Sig,
}

impl OperandType {
    pub fn size(self) -> usize {
        match self {
            Self::None => 0,
            Self::ShortI | Self::ShortBranch | Self::ShortVar => 1,
            Self::Var => 2,
            Self::I8 | Self::R => 8,
            Self::Switch => 4,
            _ => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlowControl {
    Next,
    Branch,
    CondBranch,
    Return,
    Throw,
    Call,
    Break,
    Meta,
}

/// Number of stack slots popped / pushed. `None` means it depends on the operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StackBehaviour {
    pub pop: Option<u8>,
    pub push: Option<u8>,
}

macro_rules! stack {
    (V) => {
        None
    };
    ($n:literal) => {
        Some($n)
    };
}

macro_rules! opcodes {
    ($($variant:ident = $value:literal, $name:literal, $operand:ident, $pop:tt, $push:tt, $flow:ident;)*) => {
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromRepr, IntoStaticStr, EnumIter)]
        #[repr(u16)]
        pub enum OpCode {
            $(
                #[strum(serialize = $name)]
                $variant = $value,
            )*
        }

        impl OpCode {
            pub fn operand_type(self) -> OperandType {
                match self {
                    $(Self::$variant => OperandType::$operand,)*
                }
            }

            pub fn stack_behaviour(self) -> StackBehaviour {
                match self {
                    $(Self::$variant => StackBehaviour { pop: stack!($pop), push: stack!($push) },)*
                }
            }

            pub fn flow_control(self) -> FlowControl {
                match self {
                    $(Self::$variant => FlowControl::$flow,)*
                }
            }
        }
    };
}

impl OpCode {
    pub fn name(self) -> &'static str {
        self.into()
    }

    pub fn value(self) -> u16 {
        self as u16
    }

    pub fn is_two_byte(self) -> bool {
        (self as u16) >= 0xFE00
    }

    /// Encoded size of the opcode and its operand (switch counts only the length field).
    pub fn size(self) -> usize {
        (if self.is_two_byte() { 2 } else { 1 }) + self.operand_type().size()
    }

    /// The long form of a short branch (`br.s` -> `br`).
    pub fn long_branch(self) -> Option<OpCode> {
        Some(match self {
            Self::Br_S => Self::Br,
            Self::Brfalse_S => Self::Brfalse,
            Self::Brtrue_S => Self::Brtrue,
            Self::Beq_S => Self::Beq,
            Self::Bge_S => Self::Bge,
            Self::Bgt_S => Self::Bgt,
            Self::Ble_S => Self::Ble,
            Self::Blt_S => Self::Blt,
            Self::Bne_Un_S => Self::Bne_Un,
            Self::Bge_Un_S => Self::Bge_Un,
            Self::Bgt_Un_S => Self::Bgt_Un,
            Self::Ble_Un_S => Self::Ble_Un,
            Self::Blt_Un_S => Self::Blt_Un,
            Self::Leave_S => Self::Leave,
            _ => return None,
        })
    }
}

impl std::fmt::Display for OpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[rustfmt::skip]
opcodes! {
    Nop = 0x00, "nop", None, 0, 0, Next;
    Break = 0x01, "break", None, 0, 0, Break;
    Ldarg_0 = 0x02, "ldarg.0", None, 0, 1, Next;
    Ldarg_1 = 0x03, "ldarg.1", None, 0, 1, Next;
    Ldarg_2 = 0x04, "ldarg.2", None, 0, 1, Next;
    Ldarg_3 = 0x05, "ldarg.3", None, 0, 1, Next;
    Ldloc_0 = 0x06, "ldloc.0", None, 0, 1, Next;
    Ldloc_1 = 0x07, "ldloc.1", None, 0, 1, Next;
    Ldloc_2 = 0x08, "ldloc.2", None, 0, 1, Next;
    Ldloc_3 = 0x09, "ldloc.3", None, 0, 1, Next;
    Stloc_0 = 0x0A, "stloc.0", None, 1, 0, Next;
    Stloc_1 = 0x0B, "stloc.1", None, 1, 0, Next;
    Stloc_2 = 0x0C, "stloc.2", None, 1, 0, Next;
    Stloc_3 = 0x0D, "stloc.3", None, 1, 0, Next;
    Ldarg_S = 0x0E, "ldarg.s", ShortVar, 0, 1, Next;
    Ldarga_S = 0x0F, "ldarga.s", ShortVar, 0, 1, Next;
    Starg_S = 0x10, "starg.s", ShortVar, 1, 0, Next;
    Ldloc_S = 0x11, "ldloc.s", ShortVar, 0, 1, Next;
    Ldloca_S = 0x12, "ldloca.s", ShortVar, 0, 1, Next;
    Stloc_S = 0x13, "stloc.s", ShortVar, 1, 0, Next;
    Ldnull = 0x14, "ldnull", None, 0, 1, Next;
    Ldc_I4_M1 = 0x15, "ldc.i4.m1", None, 0, 1, Next;
    Ldc_I4_0 = 0x16, "ldc.i4.0", None, 0, 1, Next;
    Ldc_I4_1 = 0x17, "ldc.i4.1", None, 0, 1, Next;
    Ldc_I4_2 = 0x18, "ldc.i4.2", None, 0, 1, Next;
    Ldc_I4_3 = 0x19, "ldc.i4.3", None, 0, 1, Next;
    Ldc_I4_4 = 0x1A, "ldc.i4.4", None, 0, 1, Next;
    Ldc_I4_5 = 0x1B, "ldc.i4.5", None, 0, 1, Next;
    Ldc_I4_6 = 0x1C, "ldc.i4.6", None, 0, 1, Next;
    Ldc_I4_7 = 0x1D, "ldc.i4.7", None, 0, 1, Next;
    Ldc_I4_8 = 0x1E, "ldc.i4.8", None, 0, 1, Next;
    Ldc_I4_S = 0x1F, "ldc.i4.s", ShortI, 0, 1, Next;
    Ldc_I4 = 0x20, "ldc.i4", I, 0, 1, Next;
    Ldc_I8 = 0x21, "ldc.i8", I8, 0, 1, Next;
    Ldc_R4 = 0x22, "ldc.r4", ShortR, 0, 1, Next;
    Ldc_R8 = 0x23, "ldc.r8", R, 0, 1, Next;
    Dup = 0x25, "dup", None, 1, 2, Next;
    Pop = 0x26, "pop", None, 1, 0, Next;
    Jmp = 0x27, "jmp", Method, 0, 0, Call;
    Call = 0x28, "call", Method, V, V, Call;
    Calli = 0x29, "calli", Sig, V, V, Call;
    Ret = 0x2A, "ret", None, V, 0, Return;
    Br_S = 0x2B, "br.s", ShortBranch, 0, 0, Branch;
    Brfalse_S = 0x2C, "brfalse.s", ShortBranch, 1, 0, CondBranch;
    Brtrue_S = 0x2D, "brtrue.s", ShortBranch, 1, 0, CondBranch;
    Beq_S = 0x2E, "beq.s", ShortBranch, 2, 0, CondBranch;
    Bge_S = 0x2F, "bge.s", ShortBranch, 2, 0, CondBranch;
    Bgt_S = 0x30, "bgt.s", ShortBranch, 2, 0, CondBranch;
    Ble_S = 0x31, "ble.s", ShortBranch, 2, 0, CondBranch;
    Blt_S = 0x32, "blt.s", ShortBranch, 2, 0, CondBranch;
    Bne_Un_S = 0x33, "bne.un.s", ShortBranch, 2, 0, CondBranch;
    Bge_Un_S = 0x34, "bge.un.s", ShortBranch, 2, 0, CondBranch;
    Bgt_Un_S = 0x35, "bgt.un.s", ShortBranch, 2, 0, CondBranch;
    Ble_Un_S = 0x36, "ble.un.s", ShortBranch, 2, 0, CondBranch;
    Blt_Un_S = 0x37, "blt.un.s", ShortBranch, 2, 0, CondBranch;
    Br = 0x38, "br", Branch, 0, 0, Branch;
    Brfalse = 0x39, "brfalse", Branch, 1, 0, CondBranch;
    Brtrue = 0x3A, "brtrue", Branch, 1, 0, CondBranch;
    Beq = 0x3B, "beq", Branch, 2, 0, CondBranch;
    Bge = 0x3C, "bge", Branch, 2, 0, CondBranch;
    Bgt = 0x3D, "bgt", Branch, 2, 0, CondBranch;
    Ble = 0x3E, "ble", Branch, 2, 0, CondBranch;
    Blt = 0x3F, "blt", Branch, 2, 0, CondBranch;
    Bne_Un = 0x40, "bne.un", Branch, 2, 0, CondBranch;
    Bge_Un = 0x41, "bge.un", Branch, 2, 0, CondBranch;
    Bgt_Un = 0x42, "bgt.un", Branch, 2, 0, CondBranch;
    Ble_Un = 0x43, "ble.un", Branch, 2, 0, CondBranch;
    Blt_Un = 0x44, "blt.un", Branch, 2, 0, CondBranch;
    Switch = 0x45, "switch", Switch, 1, 0, CondBranch;
    Ldind_I1 = 0x46, "ldind.i1", None, 1, 1, Next;
    Ldind_U1 = 0x47, "ldind.u1", None, 1, 1, Next;
    Ldind_I2 = 0x48, "ldind.i2", None, 1, 1, Next;
    Ldind_U2 = 0x49, "ldind.u2", None, 1, 1, Next;
    Ldind_I4 = 0x4A, "ldind.i4", None, 1, 1, Next;
    Ldind_U4 = 0x4B, "ldind.u4", None, 1, 1, Next;
    Ldind_I8 = 0x4C, "ldind.i8", None, 1, 1, Next;
    Ldind_I = 0x4D, "ldind.i", None, 1, 1, Next;
    Ldind_R4 = 0x4E, "ldind.r4", None, 1, 1, Next;
    Ldind_R8 = 0x4F, "ldind.r8", None, 1, 1, Next;
    Ldind_Ref = 0x50, "ldind.ref", None, 1, 1, Next;
    Stind_Ref = 0x51, "stind.ref", None, 2, 0, Next;
    Stind_I1 = 0x52, "stind.i1", None, 2, 0, Next;
    Stind_I2 = 0x53, "stind.i2", None, 2, 0, Next;
    Stind_I4 = 0x54, "stind.i4", None, 2, 0, Next;
    Stind_I8 = 0x55, "stind.i8", None, 2, 0, Next;
    Stind_R4 = 0x56, "stind.r4", None, 2, 0, Next;
    Stind_R8 = 0x57, "stind.r8", None, 2, 0, Next;
    Add = 0x58, "add", None, 2, 1, Next;
    Sub = 0x59, "sub", None, 2, 1, Next;
    Mul = 0x5A, "mul", None, 2, 1, Next;
    Div = 0x5B, "div", None, 2, 1, Next;
    Div_Un = 0x5C, "div.un", None, 2, 1, Next;
    Rem = 0x5D, "rem", None, 2, 1, Next;
    Rem_Un = 0x5E, "rem.un", None, 2, 1, Next;
    And = 0x5F, "and", None, 2, 1, Next;
    Or = 0x60, "or", None, 2, 1, Next;
    Xor = 0x61, "xor", None, 2, 1, Next;
    Shl = 0x62, "shl", None, 2, 1, Next;
    Shr = 0x63, "shr", None, 2, 1, Next;
    Shr_Un = 0x64, "shr.un", None, 2, 1, Next;
    Neg = 0x65, "neg", None, 1, 1, Next;
    Not = 0x66, "not", None, 1, 1, Next;
    Conv_I1 = 0x67, "conv.i1", None, 1, 1, Next;
    Conv_I2 = 0x68, "conv.i2", None, 1, 1, Next;
    Conv_I4 = 0x69, "conv.i4", None, 1, 1, Next;
    Conv_I8 = 0x6A, "conv.i8", None, 1, 1, Next;
    Conv_R4 = 0x6B, "conv.r4", None, 1, 1, Next;
    Conv_R8 = 0x6C, "conv.r8", None, 1, 1, Next;
    Conv_U4 = 0x6D, "conv.u4", None, 1, 1, Next;
    Conv_U8 = 0x6E, "conv.u8", None, 1, 1, Next;
    Callvirt = 0x6F, "callvirt", Method, V, V, Call;
    Cpobj = 0x70, "cpobj", Type, 2, 0, Next;
    Ldobj = 0x71, "ldobj", Type, 1, 1, Next;
    Ldstr = 0x72, "ldstr", String, 0, 1, Next;
    Newobj = 0x73, "newobj", Method, V, 1, Call;
    Castclass = 0x74, "castclass", Type, 1, 1, Next;
    Isinst = 0x75, "isinst", Type, 1, 1, Next;
    Conv_R_Un = 0x76, "conv.r.un", None, 1, 1, Next;
    Unbox = 0x79, "unbox", Type, 1, 1, Next;
    Throw = 0x7A, "throw", None, 1, 0, Throw;
    Ldfld = 0x7B, "ldfld", Field, 1, 1, Next;
    Ldflda = 0x7C, "ldflda", Field, 1, 1, Next;
    Stfld = 0x7D, "stfld", Field, 2, 0, Next;
    Ldsfld = 0x7E, "ldsfld", Field, 0, 1, Next;
    Ldsflda = 0x7F, "ldsflda", Field, 0, 1, Next;
    Stsfld = 0x80, "stsfld", Field, 1, 0, Next;
    Stobj = 0x81, "stobj", Type, 2, 0, Next;
    Conv_Ovf_I1_Un = 0x82, "conv.ovf.i1.un", None, 1, 1, Next;
    Conv_Ovf_I2_Un = 0x83, "conv.ovf.i2.un", None, 1, 1, Next;
    Conv_Ovf_I4_Un = 0x84, "conv.ovf.i4.un", None, 1, 1, Next;
    Conv_Ovf_I8_Un = 0x85, "conv.ovf.i8.un", None, 1, 1, Next;
    Conv_Ovf_U1_Un = 0x86, "conv.ovf.u1.un", None, 1, 1, Next;
    Conv_Ovf_U2_Un = 0x87, "conv.ovf.u2.un", None, 1, 1, Next;
    Conv_Ovf_U4_Un = 0x88, "conv.ovf.u4.un", None, 1, 1, Next;
    Conv_Ovf_U8_Un = 0x89, "conv.ovf.u8.un", None, 1, 1, Next;
    Conv_Ovf_I_Un = 0x8A, "conv.ovf.i.un", None, 1, 1, Next;
    Conv_Ovf_U_Un = 0x8B, "conv.ovf.u.un", None, 1, 1, Next;
    Box = 0x8C, "box", Type, 1, 1, Next;
    Newarr = 0x8D, "newarr", Type, 1, 1, Next;
    Ldlen = 0x8E, "ldlen", None, 1, 1, Next;
    Ldelema = 0x8F, "ldelema", Type, 2, 1, Next;
    Ldelem_I1 = 0x90, "ldelem.i1", None, 2, 1, Next;
    Ldelem_U1 = 0x91, "ldelem.u1", None, 2, 1, Next;
    Ldelem_I2 = 0x92, "ldelem.i2", None, 2, 1, Next;
    Ldelem_U2 = 0x93, "ldelem.u2", None, 2, 1, Next;
    Ldelem_I4 = 0x94, "ldelem.i4", None, 2, 1, Next;
    Ldelem_U4 = 0x95, "ldelem.u4", None, 2, 1, Next;
    Ldelem_I8 = 0x96, "ldelem.i8", None, 2, 1, Next;
    Ldelem_I = 0x97, "ldelem.i", None, 2, 1, Next;
    Ldelem_R4 = 0x98, "ldelem.r4", None, 2, 1, Next;
    Ldelem_R8 = 0x99, "ldelem.r8", None, 2, 1, Next;
    Ldelem_Ref = 0x9A, "ldelem.ref", None, 2, 1, Next;
    Stelem_I = 0x9B, "stelem.i", None, 3, 0, Next;
    Stelem_I1 = 0x9C, "stelem.i1", None, 3, 0, Next;
    Stelem_I2 = 0x9D, "stelem.i2", None, 3, 0, Next;
    Stelem_I4 = 0x9E, "stelem.i4", None, 3, 0, Next;
    Stelem_I8 = 0x9F, "stelem.i8", None, 3, 0, Next;
    Stelem_R4 = 0xA0, "stelem.r4", None, 3, 0, Next;
    Stelem_R8 = 0xA1, "stelem.r8", None, 3, 0, Next;
    Stelem_Ref = 0xA2, "stelem.ref", None, 3, 0, Next;
    Ldelem = 0xA3, "ldelem", Type, 2, 1, Next;
    Stelem = 0xA4, "stelem", Type, 3, 0, Next;
    Unbox_Any = 0xA5, "unbox.any", Type, 1, 1, Next;
    Conv_Ovf_I1 = 0xB3, "conv.ovf.i1", None, 1, 1, Next;
    Conv_Ovf_U1 = 0xB4, "conv.ovf.u1", None, 1, 1, Next;
    Conv_Ovf_I2 = 0xB5, "conv.ovf.i2", None, 1, 1, Next;
    Conv_Ovf_U2 = 0xB6, "conv.ovf.u2", None, 1, 1, Next;
    Conv_Ovf_I4 = 0xB7, "conv.ovf.i4", None, 1, 1, Next;
    Conv_Ovf_U4 = 0xB8, "conv.ovf.u4", None, 1, 1, Next;
    Conv_Ovf_I8 = 0xB9, "conv.ovf.i8", None, 1, 1, Next;
    Conv_Ovf_U8 = 0xBA, "conv.ovf.u8", None, 1, 1, Next;
    Refanyval = 0xC2, "refanyval", Type, 1, 1, Next;
    Ckfinite = 0xC3, "ckfinite", None, 1, 1, Next;
    Mkrefany = 0xC6, "mkrefany", Type, 1, 1, Next;
    Ldtoken = 0xD0, "ldtoken", Tok, 0, 1, Next;
    Conv_U2 = 0xD1, "conv.u2", None, 1, 1, Next;
    Conv_U1 = 0xD2, "conv.u1", None, 1, 1, Next;
    Conv_I = 0xD3, "conv.i", None, 1, 1, Next;
    Conv_Ovf_I = 0xD4, "conv.ovf.i", None, 1, 1, Next;
    Conv_Ovf_U = 0xD5, "conv.ovf.u", None, 1, 1, Next;
    Add_Ovf = 0xD6, "add.ovf", None, 2, 1, Next;
    Add_Ovf_Un = 0xD7, "add.ovf.un", None, 2, 1, Next;
    Mul_Ovf = 0xD8, "mul.ovf", None, 2, 1, Next;
    Mul_Ovf_Un = 0xD9, "mul.ovf.un", None, 2, 1, Next;
    Sub_Ovf = 0xDA, "sub.ovf", None, 2, 1, Next;
    Sub_Ovf_Un = 0xDB, "sub.ovf.un", None, 2, 1, Next;
    Endfinally = 0xDC, "endfinally", None, 0, 0, Return;
    Leave = 0xDD, "leave", Branch, 0, 0, Branch;
    Leave_S = 0xDE, "leave.s", ShortBranch, 0, 0, Branch;
    Stind_I = 0xDF, "stind.i", None, 2, 0, Next;
    Conv_U = 0xE0, "conv.u", None, 1, 1, Next;
    Arglist = 0xFE00, "arglist", None, 0, 1, Next;
    Ceq = 0xFE01, "ceq", None, 2, 1, Next;
    Cgt = 0xFE02, "cgt", None, 2, 1, Next;
    Cgt_Un = 0xFE03, "cgt.un", None, 2, 1, Next;
    Clt = 0xFE04, "clt", None, 2, 1, Next;
    Clt_Un = 0xFE05, "clt.un", None, 2, 1, Next;
    Ldftn = 0xFE06, "ldftn", Method, 0, 1, Next;
    Ldvirtftn = 0xFE07, "ldvirtftn", Method, 1, 1, Next;
    Ldarg = 0xFE09, "ldarg", Var, 0, 1, Next;
    Ldarga = 0xFE0A, "ldarga", Var, 0, 1, Next;
    Starg = 0xFE0B, "starg", Var, 1, 0, Next;
    Ldloc = 0xFE0C, "ldloc", Var, 0, 1, Next;
    Ldloca = 0xFE0D, "ldloca", Var, 0, 1, Next;
    Stloc = 0xFE0E, "stloc", Var, 1, 0, Next;
    Localloc = 0xFE0F, "localloc", None, 1, 1, Next;
    Endfilter = 0xFE11, "endfilter", None, 1, 0, Return;
    Unaligned = 0xFE12, "unaligned.", ShortI, 0, 0, Meta;
    Volatile = 0xFE13, "volatile.", None, 0, 0, Meta;
    Tail = 0xFE14, "tail.", None, 0, 0, Meta;
    Initobj = 0xFE15, "initobj", Type, 1, 0, Next;
    Constrained = 0xFE16, "constrained.", Type, 0, 0, Meta;
    Cpblk = 0xFE17, "cpblk", None, 3, 0, Next;
    Initblk = 0xFE18, "initblk", None, 3, 0, Next;
    No = 0xFE19, "no.", ShortI, 0, 0, Meta;
    Rethrow = 0xFE1A, "rethrow", None, 0, 0, Throw;
    Sizeof = 0xFE1C, "sizeof", Type, 0, 1, Next;
    Refanytype = 0xFE1D, "refanytype", None, 1, 1, Next;
    Readonly = 0xFE1E, "readonly.", None, 0, 0, Meta;
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn names_and_values_round_trip() {
        assert_eq!(OpCode::Ldc_I4_S.name(), "ldc.i4.s");
        assert_eq!(OpCode::from_repr(0xFE0C), Some(OpCode::Ldloc));
        for op in OpCode::iter() {
            assert_eq!(OpCode::from_repr(op.value()), Some(op));
        }
        assert_eq!(OpCode::iter().count(), 219);
    }
}
