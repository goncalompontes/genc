use super::{Expr, Ident, Item, RBlock, RExpr, RStmt, Type};

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
