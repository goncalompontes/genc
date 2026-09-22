use crate::semantic::parser::token::Literal;

use super::{Ident, RBlock, RExpr};

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
