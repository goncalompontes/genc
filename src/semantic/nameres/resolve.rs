use itertools::Itertools;

use crate::semantic::ast;

use super::{Env, FnId, ResolutionError, TyDef, TyId};

enum HoistedItem<'ast> {
    TypeDef(&'ast ast::TypeDef<'ast>, TyId),
    Alias(&'ast ast::RAlias<'ast>, TyId),
    Function(&'ast ast::RFunction<'ast>, FnId),
}

impl<'ast> Env<'ast> {
    // every time we enter a new scope we need to first look for definitions and hoist them.
    fn hoist_defs(&mut self, items: &'ast [ast::Item<'ast>]) {
        // We walk all the items, assign them an id, and define them in the current scope.
        // If a duplicate is found, we report it as a `DuplicateTypeDefinition` error.
        let hoisted = self.hoist_names(items);

        // Now that we have defined all the items, we can resolve references.
        // If a reference cannot be resolved, we report it as an `UndefinedSymbol` error.
        self.resolve_defs(hoisted);
    }

    fn hoist_names(&mut self, items: &'ast [ast::Item<'ast>]) -> Vec<HoistedItem<'ast>> {
        items
            .iter()
            .map(|item| match item {
                ast::Item::TypeDef(type_def) => {
                    let id = self.hoist_type_def(type_def);
                    HoistedItem::TypeDef(type_def, id)
                }
                ast::Item::Alias(alias) => {
                    let id = self.hoist_alias(alias);
                    HoistedItem::Alias(alias, id)
                }
                ast::Item::Function(func) => {
                    let id = self.hoist_function(func);
                    HoistedItem::Function(func, id)
                }
            })
            .collect()
    }

    fn hoist_type_def(&mut self, type_def: &'ast ast::TypeDef<'ast>) -> TyId {
        let id = self.defs.alloc_ty(type_def.name().span());
        if let Err(existing) = self
            .scopes
            .current_scope()
            .define_ty(**type_def.name(), id)
        {
            let original = self.defs.ty_span(existing);
            self.diagnostics
                .push(ResolutionError::DuplicateTypeDefinition {
                    symbol: **type_def.name(),
                    original,
                    duplicate: type_def.name().span(),
                });
        };
        id
    }

    fn hoist_alias(&mut self, alias: &'ast ast::RAlias<'ast>) -> TyId {
        let id = self.defs.alloc_ty(alias.name.span());
        if let Err(existing) = self.scopes.current_scope().define_ty(*alias.name, id) {
            let original = self.defs.ty_span(existing);
            self.diagnostics
                .push(ResolutionError::DuplicateTypeDefinition {
                    symbol: *alias.name,
                    original,
                    duplicate: alias.name.span(),
                });
        };
        id
    }

    fn hoist_function(&mut self, func: &'ast ast::RFunction<'ast>) -> FnId {
        let id = self.defs.alloc_fn(func.name.span());
        if let Err(existing) = self.scopes.current_scope().define_fn(*func.name, id) {
            let original = self.defs.fn_span(existing);
            self.diagnostics
                .push(ResolutionError::DuplicateFunctionDefinition {
                    symbol: *func.name,
                    original,
                    duplicate: func.name.span(),
                });
        }
        id
    }

    fn resolve_defs(&mut self, items: Vec<HoistedItem<'ast>>) {
        items.into_iter().for_each(|item| match item {
            HoistedItem::TypeDef(type_def, id) => self.resolve_type_def(type_def, id),
            HoistedItem::Alias(alias, id) => self.resolve_alias(alias, id),
            HoistedItem::Function(func, id) => self.resolve_function(func, id),
        });
    }

    fn resolve_type_def(&mut self, type_def: &'ast ast::TypeDef<'ast>, id: TyId) {
        match type_def {
            ast::TypeDef::Enum(enum_def) => self.resolve_enum(enum_def, id),
            ast::TypeDef::Data(data_def) => self.resolve_data(data_def, id),
        }
    }

    fn resolve_enum(&mut self, enum_def: &'ast ast::REnumDef<'ast>, id: TyId) {
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

    fn resolve_data(&mut self, data_def: &'ast ast::RDataDef<'ast>, id: TyId) {
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

    fn resolve_alias(&mut self, alias: &'ast ast::RAlias<'ast>, id: TyId) {
        todo!()
    }

    fn resolve_function(&mut self, func: &'ast ast::RFunction<'ast>, id: FnId) {
        todo!()
    }
}
