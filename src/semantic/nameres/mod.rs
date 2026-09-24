mod defs;
mod error;
mod resolutions;
mod resolve;
mod scope;
mod ty;

pub use defs::{Definitions, FnDef, FnId, TyDef, TyId, VarDef, VarId};
pub use error::ResolutionError;
pub use resolutions::Resolutions;
pub use scope::{Scope, ScopeKind, Scopes, SymTable};
pub use ty::{BuiltinTy, Ty};

use crate::semantic::ast;
use crate::semantic::diagnostics::Diagnostics;

pub struct Env<'ast> {
    scopes: Scopes,
    defs: Definitions,
    resolutions: Resolutions<'ast>,
    diagnostics: Diagnostics<ResolutionError>,
}

impl<'ast> Default for Env<'ast> {
    fn default() -> Self {
        Self::new()
    }
}
