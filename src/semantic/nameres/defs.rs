use index_vec::{IndexVec, define_index_type};

use crate::semantic::ast::Symbol;
use crate::semantic::nameres::ty::Ty;
use crate::semantic::node::{Span, Spanned};

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
    Struct { fields: Vec<(Symbol, Ty)> },
    Alias(Ty),
}
pub enum VarDef {
    Arg,
    Local,
}

pub struct FnDef {
    pub args: Box<[(Symbol, Ty)]>,
    pub ret: Ty,
}

impl FnDef {
    pub fn signature(&self) -> Ty {
        Ty::Fn(
            Box::new(self.ret.clone()),
            self.args.iter().map(|(_, ty)| ty.clone()).collect(),
        )
    }
}

#[derive(Default)]
pub struct Definitions {
    tys: IndexVec<TyId, Spanned<Option<TyDef>>>,
    vars: IndexVec<VarId, Spanned<Option<VarDef>>>,
    fns: IndexVec<FnId, Spanned<Option<FnDef>>>,
}

impl Definitions {
    // --- allocate an id now, define it later ------------------------------
    pub fn alloc_ty(&mut self, span: Span) -> TyId {
        self.tys.push(Spanned { span, value: None })
    }
    pub fn alloc_var(&mut self, span: Span) -> VarId {
        self.vars.push(Spanned { span, value: None })
    }
    pub fn alloc_fn(&mut self, span: Span) -> FnId {
        self.fns.push(Spanned { span, value: None })
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

    // -- allocate and id and define a new variable/type/fn -----------------
    pub fn insert_ty(&mut self, span: Span, def: TyDef) -> TyId {
        self.tys.push(Spanned {
            span,
            value: Some(def),
        })
    }

    pub fn insert_var(&mut self, span: Span, def: VarDef) -> VarId {
        self.vars.push(Spanned {
            span,
            value: Some(def),
        })
    }

    pub fn insert_fn(&mut self, span: Span, def: FnDef) -> FnId {
        self.fns.push(Spanned {
            span,
            value: Some(def),
        })
    }

    // --- definition locations ---------------------------------------------
    pub fn ty_span(&self, id: TyId) -> Span {
        self.tys[id].span()
    }
    pub fn var_span(&self, id: VarId) -> Span {
        self.vars[id].span()
    }
    pub fn fn_span(&self, id: FnId) -> Span {
        self.fns[id].span()
    }

    // --- accessors ---------------------------------------------------------
    /// Panics in debug builds if `id` has not been defined yet. Release builds
    /// elide the check, so reading an undefined id there is undefined behavior.
    pub fn tys(&self, id: TyId) -> &TyDef {
        Self::defined(&self.tys[id], id)
    }
    pub fn vars(&self, id: VarId) -> &VarDef {
        Self::defined(&self.vars[id], id)
    }
    pub fn fns(&self, id: FnId) -> &FnDef {
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
