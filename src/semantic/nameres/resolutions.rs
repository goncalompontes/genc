use rustc_hash::FxHashMap;

use crate::semantic::ast;
use crate::semantic::node::Id;

use super::{FnId, TyId, VarId};

#[derive(Default)]
pub struct Resolutions<'ast> {
    pub tys: FxHashMap<Id<&'ast ast::Type<'ast>>, TyId>,
    pub vars: FxHashMap<Id<&'ast ast::Expr<'ast>>, VarId>,
    pub fns: FxHashMap<Id<&'ast ast::Expr<'ast>>, FnId>,
}
