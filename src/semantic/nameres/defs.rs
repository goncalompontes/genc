use index_vec::{IndexVec, define_index_type};

use crate::semantic::ast::Symbol;
use crate::semantic::node::Span;

define_index_type! {
    pub struct TyId = usize;
}

define_index_type! {
    pub struct VarId = usize;
}

define_index_type! {
    pub struct FnId = usize;
}

pub enum TyDef {
    Enum { variants: Box<[Symbol]> },
    Struct { fields: Vec<(Symbol, TyId)> },
}
pub type VarDef = ();
pub type FnDef = ();

#[derive(Default)]
pub struct Definitions {
    tys: IndexVec<TyId, Option<TyDef>>,
    ty_spans: IndexVec<TyId, Span>,
    vars: IndexVec<VarId, Option<VarDef>>,
    var_spans: IndexVec<VarId, Span>,
    fns: IndexVec<FnId, Option<FnDef>>,
    fn_spans: IndexVec<FnId, Span>,
}

impl Definitions {
    // --- allocate an id now, define it later ------------------------------
    pub fn alloc_ty(&mut self, span: Span) -> TyId {
        self.ty_spans.push(span);
        self.tys.push(None)
    }
    pub fn alloc_var(&mut self, span: Span) -> VarId {
        self.var_spans.push(span);
        self.vars.push(None)
    }
    pub fn alloc_fn(&mut self, span: Span) -> FnId {
        self.fn_spans.push(span);
        self.fns.push(None)
    }

    // --- fill a previously allocated id -----------------------------------
    pub fn define_ty(&mut self, id: TyId, def: TyDef) {
        let old = self.tys[id].replace(def);
        debug_assert!(old.is_none(), "type {id:?} was defined twice");
    }
    pub fn define_var(&mut self, id: VarId, def: VarDef) {
        let old = self.vars[id].replace(def);
        debug_assert!(old.is_none(), "var {id:?} was defined twice");
    }
    pub fn define_fn(&mut self, id: FnId, def: FnDef) {
        let old = self.fns[id].replace(def);
        debug_assert!(old.is_none(), "fn {id:?} was defined twice");
    }

    // --- definition locations ---------------------------------------------
    pub fn ty_span(&self, id: TyId) -> Span {
        self.ty_spans[id]
    }
    pub fn var_span(&self, id: VarId) -> Span {
        self.var_spans[id]
    }
    pub fn fn_span(&self, id: FnId) -> Span {
        self.fn_spans[id]
    }

    // --- accessors ---------------------------------------------------------
    /// Panics in debug builds if `id` has not been defined yet. Release builds
    /// elide the check, so reading an undefined id there is undefined behavior.
    pub fn ty(&self, id: TyId) -> &TyDef {
        Self::defined(&self.tys[id], id)
    }
    pub fn var(&self, id: VarId) -> &VarDef {
        Self::defined(&self.vars[id], id)
    }
    pub fn fn_(&self, id: FnId) -> &FnDef {
        Self::defined(&self.fns[id], id)
    }

    /// Fetches a definition that is expected to already have been filled in.
    ///
    /// Resolution allocates every id before it computes any definition, so an id
    /// is only ever read back after it has been defined. Debug builds check that
    /// invariant and panic on violation; release builds compile the check out
    /// entirely, so reading an unfilled id is undefined behavior.
    #[cfg(debug_assertions)]
    #[inline(always)]
    #[track_caller]
    fn defined<'a, T>(slot: &'a Option<T>, id: impl std::fmt::Debug) -> &'a T {
        slot.as_ref()
            .unwrap_or_else(|| panic!("{id:?} used before it was defined"))
    }

    #[cfg(not(debug_assertions))]
    #[inline(always)]
    fn defined<'a, T>(slot: &'a Option<T>, _id: impl std::fmt::Debug) -> &'a T {
        // SAFETY: the caller upholds the invariant documented above.
        unsafe { slot.as_ref().unwrap_unchecked() }
    }
}
