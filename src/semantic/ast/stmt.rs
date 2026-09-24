use crate::semantic::ast::RType;

use super::{Expr, Ident, Item, RBlock, RExpr, RStmt, Type};

pub struct Block<'ast> {
    pub items: Vec<Item<'ast>>,
    pub stmts: Vec<RStmt<'ast>>,
    pub tail: Option<RExpr<'ast>>,
}

pub enum Stmt<'ast> {
    Expr(RExpr<'ast>),
    Let {
        name: Ident,
        ty: Option<RType<'ast>>,
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
