use std::borrow::Cow;
use std::marker::PhantomData;

pub enum Literal<'ast> {
    Int(u64),
    Float(f64),
    Bool(bool),
    String(Cow<'ast, str>),
    Char(char),
}

pub enum Token<'ast> {
    Phantom(PhantomData<&'ast str>),
    Literal(Literal<'ast>),
}
