mod defs;
mod error;
mod resolutions;
mod resolve;
mod scope;

pub use defs::{Definitions, FnDef, FnId, TyDef, TyId, VarDef, VarId};
pub use error::ResolutionError;
pub use resolutions::Resolutions;
pub use scope::{Scope, ScopeKind, Scopes, SymTable};

use crate::semantic::ast;
use crate::semantic::diagnostics::Diagnostics;

pub struct Env<'ast> {
    scopes: Scopes,
    defs: Definitions,
    resolutions: Resolutions<'ast>,
    diagnostics: Diagnostics<ResolutionError>,
}

impl<'ast> Env<'ast> {
    pub fn resolve(prog: &'ast ast::Program<'ast>) -> Self {
        Self {
            scopes: Scopes::new(),
            defs: Definitions::default(),
            resolutions: Resolutions::default(),
            diagnostics: Diagnostics::default(),
        }
    }
}
