use std::collections::hash_map::Entry;

use index_vec::{IndexVec, define_index_type};
use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet, FxHasher};

use crate::ast::{self, Symbol};
use crate::node::{Id, Span, Spanned};
use crate::semantic::error::Diagnostics;

define_index_type! {
    pub struct TyId = usize;
}

define_index_type! {
    pub struct VarId = usize;
}

define_index_type! {
    pub struct FnId = usize;
}

pub enum ScopeKind {
    Function,
    Block,
}

// note: due to the symbol table being confined to a single scope,
// we can think about replacing it with a Vec<(Symbol, VarId)> as that might actually be faster,
// specially if we guarantee that it is sorted, in which case we just do a binary search.
// (test and benchmark later on)
#[derive(Default)]
pub struct SymTable {
    pub vars: FxHashMap<Symbol, Spanned<VarId>>,
    pub fns: FxHashMap<Symbol, Spanned<FnId>>,
    pub tys: FxHashMap<Symbol, Spanned<TyId>>,
}

impl SymTable {
    pub fn new() -> Self {
        Self {
            vars: FxHashMap::default(),
            fns: FxHashMap::default(),
            tys: FxHashMap::default(),
        }
    }

    pub fn with(
        vars: FxHashMap<Symbol, Spanned<VarId>>,
        fns: FxHashMap<Symbol, Spanned<FnId>>,
        tys: FxHashMap<Symbol, Spanned<TyId>>,
    ) -> Self {
        Self { vars, fns, tys }
    }

    pub fn lookup_var(&self, name: &Symbol) -> Option<Spanned<VarId>> {
        self.vars.get(name).copied()
    }

    pub fn lookup_type(&self, name: &Symbol) -> Option<Spanned<TyId>> {
        self.tys.get(name).copied()
    }

    pub fn lookup_fn(&self, name: &Symbol) -> Option<Spanned<FnId>> {
        self.fns.get(name).copied()
    }

    /// Bind `name` to `id`, allowing it to shadow an existing binding.
    ///
    /// - `None` — the name was free; `id` is now bound.
    /// - `Some(previous)` — the old binding was replaced and is returned.
    pub fn define_var(&mut self, name: Symbol, id: Spanned<VarId>) -> Option<Spanned<VarId>> {
        self.vars.insert(name, id)
    }

    /// Bind `name` to `id`, refusing to overwrite an existing binding.
    ///
    /// - `Ok(())` — the name was free; `id` is now bound.
    /// - `Err(existing)` — the name was already bound; the map is unchanged
    ///   and `id` was *not* inserted.
    #[must_use = "the shadowed definition is returned; ignoring it hides accidental shadowing"]
    pub fn define_fn(&mut self, name: Symbol, id: Spanned<FnId>) -> Result<(), Spanned<FnId>> {
        match self.fns.entry(name) {
            Entry::Occupied(e) => Err(*e.get()),
            Entry::Vacant(e) => {
                e.insert(id);
                Ok(())
            }
        }
    }

    /// Bind `name` to `id`, refusing to overwrite an existing binding.
    ///
    /// Same contract as [`Self::define_fn`].
    #[must_use = "the shadowed definition is returned; ignoring it hides accidental shadowing"]
    pub fn define_ty(&mut self, name: Symbol, id: Spanned<TyId>) -> Result<(), Spanned<TyId>> {
        match self.tys.entry(name) {
            Entry::Occupied(e) => Err(*e.get()),
            Entry::Vacant(e) => {
                e.insert(id);
                Ok(())
            }
        }
    }
}

pub struct Scope {
    pub kind: ScopeKind,
    pub table: SymTable,
}

impl Scope {
    pub fn new(kind: ScopeKind) -> Self {
        Self {
            kind,
            table: SymTable::default(),
        }
    }

    pub fn lookup_var(&self, name: &Symbol) -> Option<VarId> {
        self.table.lookup_var(name).map(|id| id.value)
    }

    pub fn lookup_type(&self, name: &Symbol) -> Option<TyId> {
        self.table.lookup_type(name).map(|id| id.value)
    }

    pub fn lookup_fn(&self, name: &Symbol) -> Option<FnId> {
        self.table.lookup_fn(name).map(|id| id.value)
    }
}

#[derive(Default)]
pub struct Scopes {
    pub root: SymTable,
    pub scopes: Vec<Scope>,
}

impl Scopes {
    pub fn new() -> Self {
        Self::with(SymTable::default())
    }

    pub fn with(root: SymTable) -> Self {
        Self {
            root,
            scopes: Vec::new(),
        }
    }

    pub fn lookup_var(&self, name: &Symbol) -> Option<VarId> {
        let mut curr = self.scopes.iter().rev();
        loop {
            match curr.next() {
                None => break self.root.lookup_var(name).map(|id| id.value),
                Some(scope) => match scope.lookup_var(name) {
                    // name found in this scope, return its id
                    Some(id) => break Some(id),
                    // name not found in this scope, try the parent one depending on the kind of the current one
                    None => match scope.kind {
                        // If the current scope is a function, we cannot look up variables
                        // in the parent scope, since variables declared in the parent scope are not visible in the current scope
                        ScopeKind::Function => break None,
                        // If the current scope is a block, we can look up variables in the parent scope
                        ScopeKind::Block => continue,
                    },
                },
            }
        }
    }

    pub fn lookup_type(&self, name: &Symbol) -> Option<TyId> {
        let mut curr = self.scopes.iter().rev();
        loop {
            match curr.next() {
                None => break self.root.lookup_type(name).map(|id| id.value),
                Some(scope) => match scope.lookup_type(name) {
                    // type found in this scope, return its id
                    Some(id) => break Some(id),
                    // type not found in this scope, try the parent one
                    None => continue,
                },
            }
        }
    }

    pub fn lookup_fn(&self, name: &Symbol) -> Option<FnId> {
        let mut curr = self.scopes.iter().rev();
        loop {
            match curr.next() {
                None => break self.root.lookup_fn(name).map(|id| id.value),
                Some(scope) => match scope.lookup_fn(name) {
                    // function found in this scope, return its id
                    Some(id) => break Some(id),
                    // function not found in this scope, try the parent one
                    None => continue,
                },
            }
        }
    }

    pub fn current_scope(&mut self) -> &mut SymTable {
        self.scopes
            .last_mut()
            .map(|scope| &mut scope.table)
            .unwrap_or(&mut self.root)
    }
}

pub enum TyDef {
    Enum { variants: Box<[Symbol]> },
    Struct { fields: Vec<(Symbol, TyId)> },
}
pub type VarDef = ();
pub type FnDef = ();

#[derive(Default)]
pub struct Definitions {
    pub tys: IndexVec<TyId, Option<TyDef>>,
    pub vars: IndexVec<VarId, Option<VarDef>>,
    pub fns: IndexVec<FnId, Option<FnDef>>,
}

/// Fetches a definition that is expected to already have been filled in.
///
/// Resolution allocates every id before it computes any definition, so an id is
/// only ever read back after it has been defined. Debug builds check that
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

impl Definitions {
    // --- allocate an id now, define it later ------------------------------
    pub fn alloc_ty(&mut self) -> TyId {
        self.tys.push(None)
    }
    pub fn alloc_var(&mut self) -> VarId {
        self.vars.push(None)
    }
    pub fn alloc_fn(&mut self) -> FnId {
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

    // --- accessors ---------------------------------------------------------
    /// Panics in debug builds if `id` has not been defined yet. Release builds
    /// elide the check, so reading an undefined id there is undefined behavior.
    pub fn ty(&self, id: TyId) -> &TyDef {
        defined(&self.tys[id], id)
    }
    pub fn var(&self, id: VarId) -> &VarDef {
        defined(&self.vars[id], id)
    }
    pub fn fn_(&self, id: FnId) -> &FnDef {
        defined(&self.fns[id], id)
    }
}

#[derive(Default)]
pub struct Resolutions<'ast> {
    pub tys: FxHashMap<Id<&'ast ast::Type<'ast>>, TyId>,
    pub vars: FxHashMap<Id<&'ast ast::Expr<'ast>>, VarId>,
    pub fns: FxHashMap<Id<&'ast ast::Expr<'ast>>, FnId>,
}

pub enum ResolutionError {
    DuplicateTypeDefinition {
        symbol: Symbol,
        original: Span,
        duplicate: Span,
    },
    DuplicateFunctionDefinition {
        symbol: Symbol,
        original: Span,
        duplicate: Span,
    },
    DuplicateVariantDefinition {
        symbol: crate::node::Node<string_interner::symbol::SymbolU32>,
        original: chumsky::prelude::SimpleSpan,
        duplicate: chumsky::prelude::SimpleSpan,
    },
    DuplicateFieldDefinition {
        symbol: crate::node::Node<string_interner::symbol::SymbolU32>,
        original: chumsky::prelude::SimpleSpan,
        duplicate: chumsky::prelude::SimpleSpan,
    },
}

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

    // every time we enter a new scope we need to first look for definitions and hoist them.
    fn hoist_defs(&mut self, items: &'ast [ast::Item<'ast>]) {
        enum ItemWithId<'a, 'b> {
            TypeDef(&'a ast::TypeDef<'b>, TyId),
            Alias(&'a ast::RAlias<'b>, TyId),
            Function(&'a ast::RFunction<'b>, FnId),
        }

        // We walk all the items, assign them an id, and define them in the current scope.
        // If a duplicate is found, we report it as a `DuplicateTypeDefinition` error.
        items
            .iter()
            .map(|item| match item {
                ast::Item::TypeDef(type_def) => {
                    let id = self.defs.alloc_ty();
                    if let Err(existing) = self
                        .scopes
                        .current_scope()
                        .define_ty(**type_def.name(), Spanned::new(type_def.name().span(), id))
                    {
                        self.diagnostics
                            .push(ResolutionError::DuplicateTypeDefinition {
                                symbol: **type_def.name(),
                                original: existing.span(),
                                duplicate: type_def.name().span(),
                            });
                    };
                    ItemWithId::TypeDef(type_def, id)
                }
                ast::Item::Alias(alias) => {
                    let id = self.defs.alloc_ty();
                    if let Err(existing) = self
                        .scopes
                        .current_scope()
                        .define_ty(*alias.name, Spanned::new(alias.name.span(), id))
                    {
                        self.diagnostics
                            .push(ResolutionError::DuplicateTypeDefinition {
                                symbol: *alias.name,
                                original: existing.span(),
                                duplicate: alias.name.span(),
                            });
                    };
                    ItemWithId::Alias(alias, id)
                }
                ast::Item::Function(func) => {
                    let id = self.defs.alloc_fn();
                    if let Err(existing) = self
                        .scopes
                        .current_scope()
                        .define_fn(*func.name, Spanned::new(func.name.span(), id))
                    {
                        self.diagnostics
                            .push(ResolutionError::DuplicateFunctionDefinition {
                                symbol: *func.name,
                                original: existing.span(),
                                duplicate: func.name.span(),
                            });
                    }
                    ItemWithId::Function(func, id)
                }
            })
            .collect::<Vec<_>>()
            .into_iter()
            // Now that we have defined all the items, we can resolve references.
            // If a reference cannot be resolved, we report it as an `UndefinedSymbol` error.
            .for_each(|item| match item {
                ItemWithId::TypeDef(type_def, id) => match type_def {
                    ast::TypeDef::Enum(enum_def) => {
                        // We need to get only unique variants inside the enum,
                        // visiting all duplicates for error reporting
                        let variants = enum_def.unique_variants(|orig, dup| {
                            self.diagnostics
                                .push(ResolutionError::DuplicateVariantDefinition {
                                    symbol: *dup,
                                    original: orig.span(),
                                    duplicate: dup.span(),
                                });
                        });
                        // now we simply construct the definition
                        self.defs.define_ty(
                            id,
                            TyDef::Enum {
                                variants: variants.into_boxed_slice(),
                            },
                        );
                    }
                    ast::TypeDef::Data(data_def) => {
                        // note: since we already exclude duplicate fields, name res is only done
                        //  for the type of the first definition of a field.
                        //  this should probably be changed so that we *first* do lookup on the types
                        //  and only then check for duplicated so that we get better diagnostics
                        let fields = data_def
                            .unique_fields(|orig, dup| {
                                self.diagnostics
                                    .push(ResolutionError::DuplicateFieldDefinition {
                                        symbol: *dup,
                                        original: orig.span(),
                                        duplicate: dup.span(),
                                    });
                            })
                            .into_iter()
                            // now that we have all the unique fields, we lookup the types
                            .map(|(name, ty)| self.scopes.current_scope().lookup_type(&name));

                        // TODO!
                        //self.defs.define_ty(id, TyDef::Struct { fields: fields });
                    }
                },
                ItemWithId::Alias(alias, id) => todo!(),
                ItemWithId::Function(func, id) => todo!(),
            });
    }
}

pub fn resolve(prog: &ast::Program) {
    let scopes = Scopes::new();
}
