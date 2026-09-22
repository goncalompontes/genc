use chumsky::span::SimpleSpan;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHasher};

use crate::semantic::node::{self, Node, Ref};

mod defs;
mod expr;
mod stmt;
mod ty;

pub use defs::{Alias, DataDef, EnumDef, Function, TypeDef};
pub use expr::{BinOp, Expr, UnOp};
pub use stmt::{Block, Stmt};
pub use ty::Type;

pub type Symbol = string_interner::DefaultSymbol;
pub type Ident = Node<Symbol>;

pub type RType<'ast> = Ref<'ast, Type<'ast>>;
pub type RExpr<'ast> = Ref<'ast, Expr<'ast>>;
pub type RStmt<'ast> = Ref<'ast, Stmt<'ast>>;
pub type RBlock<'ast> = Ref<'ast, Block<'ast>>;
pub type REnumDef<'ast> = Ref<'ast, EnumDef<'ast>>;
pub type RDataDef<'ast> = Ref<'ast, DataDef<'ast>>;
pub type RAlias<'ast> = Ref<'ast, Alias<'ast>>;
pub type RFunction<'ast> = Ref<'ast, Function<'ast>>;

// missing:
// `variant type <ident> = ...`
// `data type <ident> = ...`
// `alias type <ident> = <Type>`
// `import a::b::...`
// type paths
// completing bin and un ops

pub struct Program<'ast> {
    pub items: Vec<Item<'ast>>,
}

pub enum Item<'ast> {
    TypeDef(TypeDef<'ast>),
    Alias(RAlias<'ast>),
    Function(RFunction<'ast>),
}

#[derive(Debug, Clone, Copy)]
pub enum Visibility {
    Pub,
    Priv,
}
