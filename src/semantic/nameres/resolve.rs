use itertools::Itertools;

use crate::semantic::ast;
use crate::semantic::node::Spanned;

use super::{Env, FnId, ResolutionError, TyDef, TyId};

impl<'ast> Env<'ast> {
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
