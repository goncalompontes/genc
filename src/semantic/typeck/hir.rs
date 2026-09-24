use index_vec::{IndexVec, define_index_type};
use rustc_hash::FxHashMap;

use crate::semantic::ast::{BinOp, UnOp};
use crate::semantic::nameres::{FnId, Ty, VarId};

define_index_type! {
    pub struct ExprId = usize;
}

define_index_type! {
    pub struct StmtId = usize;
}

pub struct Module {
    pub functions: FxHashMap<FnId, Block>,
    pub exprs: IndexVec<ExprId, Expr>,
}

pub struct Block {
    pub stmts: Vec<Stmt>,
    pub tail: Option<ExprId>,
}

pub enum Stmt {
    Expr(ExprId),
    Let {
        name: VarId,
        ty: Ty,
        value: ExprId,
    },
    Assign {
        place: ExprId,
        value: ExprId,
    },
    Return {
        value: Option<ExprId>,
    },
    Loop {
        body: Block,
    },
    Break {
        value: Option<ExprId>,
    },
    Continue,
    If {
        cond: ExprId,
        then_block: Block,
        else_block: Option<Block>,
    },
}

pub enum Literal {
    Int(u64),
    Float(f64),
    Bool(bool),
    String(String),
    Char(char),
}

pub enum Expr {
    Literal(Literal),
    Variable(VarId),
    Function(FnId),
    Grouping(ExprId),
    Binary { lhs: ExprId, rhs: ExprId, op: BinOp },
    Unary { rhs: ExprId, op: UnOp },
    Field { lhs: ExprId, index: usize },
    Call { lhs: ExprId, args: Vec<ExprId> },
    Index { lhs: ExprId, index: ExprId },
    Ref(ExprId),
    Deref(ExprId),
    Block(Block),
}
