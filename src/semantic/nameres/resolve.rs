use itertools::{Either, Itertools};

use crate::semantic::ast::{self, Ident, Symbol};
use crate::semantic::diagnostics::Diagnostics;
use crate::semantic::nameres::ty::Ty;
use crate::semantic::nameres::{Definitions, FnDef, Resolutions, ScopeKind, Scopes, VarDef};
use crate::semantic::node::Span;

use super::{Env, FnId, ResolutionError, TyDef, TyId};

enum HoistedItem<'ast> {
    TypeDef(&'ast ast::TypeDef<'ast>, TyId),
    Alias(&'ast ast::RAlias<'ast>, TyId),
    Function(&'ast ast::RFunction<'ast>, FnId),
}

impl<'ast> Env<'ast> {
    pub fn new() -> Self {
        Self {
            scopes: Scopes::new(),
            defs: Definitions::default(),
            resolutions: Resolutions::default(),
            diagnostics: Diagnostics::default(),
        }
    }

    pub fn resolve(&mut self, prog: &'ast ast::Program<'ast>) {
        self.resolve_items(&prog.items);
    }

    fn emit_unresolved_name(&mut self, name: &Ident) {
        self.diagnostics.push(ResolutionError::UnknownName {
            span: name.span(),
            name: **name,
        });
    }

    fn emit_unknown_type(&mut self, name: &Ident) {
        self.diagnostics.push(ResolutionError::UnknownType {
            span: name.span(),
            name: **name,
        });
    }

    fn resolve_type(&mut self, ty: &ast::RType<'ast>, f: impl Fn(&mut Self, &Ident)) -> Ty {
        let resolved_ty = match &**ty {
            ast::Type::Named(node) => self.scopes.lookup_type(node).unwrap_or_else(|| {
                f(self, node);
                Ty::Unknown
            }),
            ast::Type::Pointer(node) => Ty::Pointer(Box::new(self.resolve_type(node, f))),
            ast::Type::Array(node) => Ty::Array(Box::new(self.resolve_type(node, f))),
            ast::Type::Fn(node, nodes) => Ty::Fn(
                Box::new(self.resolve_type(node, &f)),
                nodes.iter().map(|n| self.resolve_type(n, &f)).collect(),
            ),
        };
        self.resolutions.tys.insert(ty.id(), resolved_ty.clone());
        resolved_ty
    }

    // every time we enter a new scope we need to first look for definitions and hoist them.
    fn resolve_items(&mut self, items: &'ast [ast::Item<'ast>]) {
        // We walk all the items, assign them an id, and define them in the current scope.
        // If a duplicate is found, we report it as a `DuplicateTypeDefinition` error.
        let declared = self.declare_items(items);

        // Now that we have defined all the items, we can resolve references.
        // If a reference cannot be resolved, we report it as an `UndefinedSymbol` error.
        self.define_items(declared.into_iter());
    }

    fn declare_items(&mut self, items: &'ast [ast::Item<'ast>]) -> Vec<HoistedItem<'ast>> {
        items
            .iter()
            .map(|item| match item {
                ast::Item::TypeDef(type_def) => {
                    let id = self.declare_type_def(type_def);
                    HoistedItem::TypeDef(type_def, id)
                }
                ast::Item::Alias(alias) => {
                    let id = self.declare_alias(alias);
                    HoistedItem::Alias(alias, id)
                }
                ast::Item::Function(func) => {
                    let id = self.declare_function(func);
                    HoistedItem::Function(func, id)
                }
            })
            .collect()
    }

    fn type_span(&self, ty: Ty) -> Option<Span> {
        match ty {
            Ty::Custom(id) => Some(self.defs.ty_span(id)),
            _ => None,
        }
    }

    fn declare_type_def(&mut self, type_def: &ast::TypeDef<'ast>) -> TyId {
        let id = self.defs.alloc_ty(type_def.name().span());
        if let Err(existing) = self
            .scopes
            .current_scope()
            .define_ty(**type_def.name(), Ty::Custom(id))
        {
            self.diagnostics
                .push(ResolutionError::DuplicateTypeDefinition {
                    symbol: **type_def.name(),
                    original: self.type_span(existing),
                    duplicate: type_def.name().span(),
                });
        };
        id
    }

    fn declare_alias(&mut self, alias: &ast::RAlias<'ast>) -> TyId {
        let id = self.defs.alloc_ty(alias.name.span());
        if let Err(existing) = self
            .scopes
            .current_scope()
            .define_ty(*alias.name, Ty::Custom(id))
        {
            self.diagnostics
                .push(ResolutionError::DuplicateTypeDefinition {
                    symbol: *alias.name,
                    original: self.type_span(existing),
                    duplicate: alias.name.span(),
                });
        };
        id
    }

    fn declare_function(&mut self, func: &ast::RFunction<'ast>) -> FnId {
        let id = self.defs.alloc_fn(func.name.span());
        if let Err(existing) = self.scopes.current_scope().define_fn(*func.name, id) {
            self.diagnostics
                .push(ResolutionError::DuplicateFunctionDefinition {
                    symbol: *func.name,
                    original: self.defs.fn_span(existing),
                    duplicate: func.name.span(),
                });
        }
        id
    }

    fn define_items<I: Iterator<Item = HoistedItem<'ast>>>(&mut self, items: I) {
        items.for_each(|item| match item {
            HoistedItem::TypeDef(type_def, id) => self.define_type(type_def, id),
            HoistedItem::Alias(alias, id) => self.define_alias(alias, id),
            HoistedItem::Function(func, id) => self.define_function(func, id),
        });
    }

    fn define_type(&mut self, type_def: &ast::TypeDef<'ast>, id: TyId) {
        match type_def {
            ast::TypeDef::Enum(enum_def) => self.define_enum(enum_def, id),
            ast::TypeDef::Data(data_def) => self.define_data(data_def, id),
        }
    }

    fn define_enum(&mut self, enum_def: &ast::REnumDef<'ast>, id: TyId) {
        // We need to get only unique variants inside the enum,
        // visiting all duplicates for error reporting
        let variants = enum_def.unique_variants(|orig, dup| {
            self.diagnostics
                .push(ResolutionError::DuplicateVariantDefinition {
                    symbol: **dup,
                    original: orig.span(),
                    duplicate: dup.span(),
                });
        });
        // now we simply construct the definition
        self.defs.define_ty(
            id,
            TyDef::Enum {
                variants: variants.collect(),
            },
        );
    }

    fn define_data(&mut self, data_def: &ast::RDataDef<'ast>, id: TyId) {
        // note: since we already exclude duplicate fields, name res is only done
        //  for the type of the first definition of a field.
        //  this should probably be changed so that we *first* do lookup on the types
        //  and only then check for duplicated so that we get better diagnostics
        let fields = data_def
            .unique_fields(|orig, dup| {
                self.diagnostics
                    .push(ResolutionError::DuplicateFieldDefinition {
                        symbol: **dup,
                        original: orig.span(),
                        duplicate: dup.span(),
                    });
            })
            .collect_vec()
            .into_iter()
            // now that we have all the unique fields, we lookup the types
            .map(|(name, ty)| {
                let ty = self.resolve_type(ty, Self::emit_unknown_type);
                (*name, ty)
            })
            .collect_vec();

        self.defs.define_ty(id, TyDef::Struct { fields });
    }

    fn define_alias(&mut self, alias: &ast::RAlias<'ast>, id: TyId) {
        // for aliases all we really need to do is resolve the type
        // for now we will not canonicalize the type
        let ty = self.resolve_type(&alias.ty, Self::emit_unknown_type);
        self.defs.define_ty(id, TyDef::Alias(ty));
    }

    fn define_function(&mut self, func: &ast::RFunction<'ast>, id: FnId) {
        // for functions we need to build the function signature
        // and then define it

        let resolved_args = func.args.iter().map(|(name, ty)| {
            let ty = self.resolve_type(ty, Self::emit_unknown_type);
            (*name, ty)
        });

        let args = resolved_args.map(|(name, ty)| (*name, ty)).collect();

        let ret = self.resolve_type(&func.ret, Self::emit_unknown_type);
        let def = FnDef { args, ret };

        self.defs.define_fn(id, def);

        self.resolutions.fns.insert(func.id(), id);

        // TODO: Now that we have a definition for a function, we can recurse into its body
        // by entering a new scope and calling on resolve_items on it
        // Then, once we have all items at the block scope defined,
        // we can actually resolve variables which is the final missing part
        // of name res

        self.resolve_function(func);
    }

    fn resolve_function(&mut self, func: &ast::RFunction<'ast>) {
        self.scopes.enter(ScopeKind::Function);

        // we start by defining each of the arguments of the function in the current scope.
        func.args.iter().for_each(|(name, ty)| {
            let ty = self.resolve_type(ty, Self::emit_unknown_type);

            let id = self.defs.insert_var(name.span(), VarDef::Arg);
            self.scopes.current_scope().define_var(**name, id);
        });

        self.resolve_block(&func.body);

        self.scopes.exit();
    }

    fn resolve_block(&mut self, block: &ast::RBlock<'ast>) {
        self.resolve_items(&block.items);
        block.stmts.iter().for_each(|stmt| self.resolve_stmt(stmt));
        if let Some(node) = &block.tail {
            self.resolve_expr(node);
        }
    }

    fn resolve_stmt(&mut self, stmt: &ast::RStmt<'ast>) {
        match &**stmt {
            ast::Stmt::Expr(expr) => self.resolve_expr(expr),
            ast::Stmt::Let { name, ty, value } => {
                // first resolve the type
                let resolved_ty = ty
                    .as_ref()
                    .map(|ty| self.resolve_type(ty, Self::emit_unknown_type));
                // then resolve the expression
                self.resolve_expr(value);
                // then insert the variable
                let id = self.defs.insert_var(name.span(), VarDef::Local);
                self.scopes.current_scope().define_var(**name, id);
                self.resolutions.vars.insert(name.id(), Either::Left(id));
            }
            ast::Stmt::Assign { place, value } => {
                self.resolve_expr(value);
                self.resolve_expr(place);
            }
            ast::Stmt::Return { value } => {
                if let Some(value) = value {
                    self.resolve_expr(value);
                }
            }
            ast::Stmt::Loop { body } => {
                self.scopes.enter(ScopeKind::Block);
                self.resolve_block(body);
                self.scopes.exit();
            }
            ast::Stmt::Break { value } => {
                if let Some(value) = value {
                    self.resolve_expr(value);
                }
            }
            ast::Stmt::Continue => {}
            ast::Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                self.resolve_expr(cond);

                self.scopes.enter(ScopeKind::Block);
                self.resolve_block(then_block);
                self.scopes.exit();

                self.scopes.enter(ScopeKind::Block);
                if let Some(else_block) = else_block {
                    self.resolve_block(else_block);
                }
                self.scopes.exit();
            }
        }
    }

    fn resolve_expr(&mut self, expr: &ast::RExpr<'ast>) {
        match &**expr {
            ast::Expr::Variable(name) => {
                if let Some(id) = self.scopes.lookup_var(name) {
                    self.resolutions.vars.insert(name.id(), Either::Left(id));
                } else if let Some(id) = self.scopes.lookup_fn(name) {
                    self.resolutions.vars.insert(name.id(), Either::Right(id));
                } else {
                    self.emit_unresolved_name(name);
                }
            }
            ast::Expr::Grouping(node) => self.resolve_expr(node),
            ast::Expr::Binary { lhs, rhs, .. } => {
                self.resolve_expr(lhs);
                self.resolve_expr(rhs);
            }
            ast::Expr::Unary { rhs, .. } => {
                self.resolve_expr(rhs);
            }
            ast::Expr::Field { lhs, .. } => {
                self.resolve_expr(lhs);
            }
            ast::Expr::Call { lhs, args } => {
                self.resolve_expr(lhs);
                args.iter().for_each(|arg| self.resolve_expr(arg));
            }
            ast::Expr::MethodCall { lhs, args, .. } => {
                self.resolve_expr(lhs);
                args.iter().for_each(|arg| self.resolve_expr(arg));
            }
            ast::Expr::Index { lhs, index } => {
                self.resolve_expr(lhs);
                self.resolve_expr(index);
            }
            ast::Expr::Ref(node) => {
                self.resolve_expr(node);
            }
            ast::Expr::Deref(node) => {
                self.resolve_expr(node);
            }
            ast::Expr::Block(node) => {
                self.scopes.enter(ScopeKind::Block);
                self.resolve_block(node);
                self.scopes.exit();
            }
            ast::Expr::Literal(..) => {}
        }
    }
}
