use itertools::Either;
use rustc_hash::FxHashMap;

use crate::semantic::ast;
use crate::semantic::nameres::ty::Ty;
use crate::semantic::node::Id;

use super::{FnId, VarId};

#[derive(Default)]
pub struct Resolutions<'ast> {
    pub tys: FxHashMap<Id<&'ast ast::Type<'ast>>, Ty>,
    pub fns: FxHashMap<Id<&'ast ast::Function<'ast>>, FnId>,
    pub vars: FxHashMap<Id<ast::Symbol>, Either<VarId, FnId>>,
}
