use std::collections::hash_map::Entry;

use rustc_hash::{FxHashMap, FxHashSet, FxHasher};

use crate::semantic::ast::Symbol;
use crate::semantic::node::Spanned;

use super::{FnId, TyId, VarId};

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
