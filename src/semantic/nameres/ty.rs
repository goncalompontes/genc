use crate::semantic::nameres::TyId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinTy {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Bool,
    Char,
    String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    Builtin(BuiltinTy),
    Pointer(Box<Ty>),
    Array(Box<Ty>),
    Fn(Box<Ty>, Vec<Ty>),
    Custom(TyId),
    Unit,
    Never,
    Unknown,
}
