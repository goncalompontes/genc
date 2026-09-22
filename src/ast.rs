use std::collections::HashMap;

use chumsky::span::SimpleSpan;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHasher};

use crate::node::{self, Node, Ref};
use crate::token::Literal;

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

pub enum TypeDef<'ast> {
    Enum(REnumDef<'ast>),
    Data(RDataDef<'ast>),
}

impl<'ast> TypeDef<'ast> {
    pub fn vis(&self) -> Visibility {
        match self {
            TypeDef::Enum(enum_def) => enum_def.vis,
            TypeDef::Data(data_def) => data_def.vis,
        }
    }

    pub fn name(&self) -> &Ident {
        match self {
            TypeDef::Enum(enum_def) => &enum_def.name,
            TypeDef::Data(data_def) => &data_def.name,
        }
    }
}

pub struct Alias<'ast> {
    pub vis: Visibility,
    pub name: Ident,
    pub ty: RType<'ast>,
}

// We will keep structs as simple structs for now
pub struct DataDef<'ast> {
    pub vis: Visibility,
    pub name: Ident,
    pub fields: Vec<(Ident, RType<'ast>)>,
}

impl<'ast> DataDef<'ast> {
    pub fn unique_fields<F: FnMut(&Ident, &Ident)>(
        &self,
        mut visit_duplicate: F,
    ) -> Vec<(Ident, RType<'ast>)> {
        let mut fields: HashMap<Symbol, (Ident, RType<'ast>)> =
            HashMap::with_capacity(self.fields.len());
        for v in &self.fields {
            match fields.get(&v.0) {
                Some(orig) => {
                    visit_duplicate(&orig.0, &v.0);
                }
                None => {
                    fields.insert(*v.0, *v);
                }
            }
        }
        fields.into_values().collect()
    }
}

// We will keep enums as simple enums for now
pub struct EnumDef<'ast> {
    pub vis: Visibility,
    pub name: Ident,
    pub variants: Vec<Ident>,
    _marker: std::marker::PhantomData<&'ast ()>,
}

impl<'ast> EnumDef<'ast> {
    pub fn unique_variants<F: FnMut(&Ident, &Ident)>(&self, mut visit_duplicate: F) -> Vec<Symbol> {
        let mut variants: HashMap<Symbol, Ident> = HashMap::with_capacity(self.variants.len());
        for v in &self.variants {
            match variants.get(v) {
                Some(orig) => {
                    visit_duplicate(orig, v);
                }
                None => {
                    variants.insert(**v, *v);
                }
            }
        }
        variants.into_keys().collect()
    }
}

pub struct Function<'ast> {
    pub vis: Visibility,
    pub name: Ident,
    // generics: Vec<Generic>
    pub args: Vec<(Ident, Type<'ast>)>,
    pub ret: Type<'ast>,
    pub body: Block<'ast>,
}

pub enum Type<'ast> {
    Named(Ident),
    Pointer(RType<'ast>),
    Array(RType<'ast>),
    Fn(RType<'ast>, Vec<RType<'ast>>),
}

pub struct Block<'ast> {
    pub items: Vec<Item<'ast>>,
    pub stmts: Vec<RStmt<'ast>>,
    pub tail: Option<Expr<'ast>>,
}

pub enum Stmt<'ast> {
    Expr(Expr<'ast>),
    Let {
        name: Ident,
        ty: Type<'ast>,
        value: RExpr<'ast>,
    },
    Assign {
        place: RExpr<'ast>,
        value: RExpr<'ast>,
    },
    Return {
        value: Option<RExpr<'ast>>,
    },
    Loop {
        body: RBlock<'ast>,
    },
    Break {
        value: Option<RExpr<'ast>>,
    },
    Continue,
    If {
        cond: RExpr<'ast>,
        then_block: RBlock<'ast>,
        else_block: Option<RBlock<'ast>>,
    },
}

pub enum Expr<'ast> {
    Literal(Literal<'ast>),
    Variable(Ident),
    Grouping(RExpr<'ast>),
    Binary {
        lhs: RExpr<'ast>,
        rhs: RExpr<'ast>,
        op: BinOp,
    },
    Unary {
        rhs: RExpr<'ast>,
        op: UnOp,
    },
    Field {
        lhs: RExpr<'ast>,
        name: Ident,
    },
    Call {
        lhs: RExpr<'ast>,
        args: Vec<RExpr<'ast>>,
    },
    MethodCall {
        lhs: RExpr<'ast>,
        name: Ident,
        args: Vec<RExpr<'ast>>,
    },
    Index {
        lhs: RExpr<'ast>,
        index: RExpr<'ast>,
    },
    Ref(RExpr<'ast>),
    Deref(RExpr<'ast>),
    Block(RBlock<'ast>),
}

pub enum BinOp {
    // +
    Add,
    // -
    Sub,
    // *
    Mul,
    // /
    Div,
    // %
    Mod,
    // ==
    Eq,
    // !=
    Ne,
    // <
    Lt,
    // <=
    Le,
    // >
    Gt,
    // >=
    Ge,
    // &&
    And,
    // ||
    Or,
}
pub enum UnOp {
    // !
    Not,
    // -
    Neg,
}
