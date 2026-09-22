use super::{Ident, RType};

pub enum Type<'ast> {
    Named(Ident),
    Pointer(RType<'ast>),
    Array(RType<'ast>),
    Fn(RType<'ast>, Vec<RType<'ast>>),
}
