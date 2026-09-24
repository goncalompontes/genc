// Now that we have a fully resolved AST, we can start type checking.
// Since we don't yet support generics or any complex type features,
// the type system consists of just simple bidirectional type checking.

use std::ops::Deref;

use index_vec::IndexVec;
use itertools::Either;
use rustc_hash::FxHashMap;

use crate::semantic::ast::{BinOp, Ident, Symbol, UnOp};
use crate::semantic::diagnostics::Diagnostics;
use crate::semantic::nameres::{BuiltinTy, Definitions, Env, FnId, Resolutions, Ty, TyDef, VarId};
use crate::semantic::node::Span;
use crate::semantic::parser::token;
use crate::semantic::{ast, nameres};

use super::ast::Item::Function;

pub mod hir;

pub enum TypeError {
    UnknownField {
        ty: Ty,
        field: Span,
    },
    ArgumentCountMismatch {
        call: Span,
        func: Ty,
        count: usize,
    },
    NonIndexableType {
        span: Span,
        ty: Ty,
    },
    InvalidIndexType {
        ty: Ty,
        span: Span,
        index_ty: Ty,
    },
    InvalidDeref {
        span: Span,
        ty: Ty,
    },
    InvalidUnaryOperand {
        span: Span,
        op: UnOp,
        ty: Ty,
    },
    InvalidBinaryOperands {
        span: Span,
        op: BinOp,
        lhs: Ty,
        rhs: Ty,
    },
    MismatchedBinaryOperands {
        span: Span,
        op: BinOp,
        lhs: Ty,
        rhs: Ty,
    },
    TypeMismatch {
        span: Span,
        found: Ty,
        expected: Ty,
    },
    ArgumentTypeMismatch {
        call: Span,
        index: usize,
        found: Ty,
        expected: Ty,
    },
    NotCallable {
        span: Span,
        ty: Ty,
    },
}

pub struct Typeck<'ast> {
    env: FxHashMap<VarId, Ty>,
    resolutions: Resolutions<'ast>,
    definitions: Definitions,
    diagnostics: Diagnostics<TypeError>,
}

impl<'ast> Typeck<'ast> {
    pub fn new(resolutions: Resolutions<'ast>, definitions: Definitions) -> Self {
        Self {
            resolutions,
            definitions,
            env: FxHashMap::default(),
            diagnostics: Diagnostics::new(),
        }
    }

    pub fn check(&mut self, prog: &ast::Program<'ast>) -> hir::Module {
        let mut exprs = IndexVec::new();
        let functions = prog
            .items
            .iter()
            .filter_map(|item| match item {
                Function(func) => {
                    let id = self
                        .resolutions
                        .fns
                        .get(&func.id())
                        .copied()
                        .expect("function not found, something went wrong");
                    Some((id, self.check_function(&mut exprs, func, id)))
                }
                _ => None,
            })
            .collect();

        hir::Module { functions, exprs }
    }

    fn emit_unknown_field(&mut self, ty: Ty, field: &Ident) {
        self.diagnostics.push(TypeError::UnknownField {
            ty: ty.clone(),
            field: field.span(),
        });
    }

    fn emit_invalid_unary(&mut self, op: UnOp, ty: Ty, span: Span) {
        self.diagnostics
            .push(TypeError::InvalidUnaryOperand { span, op, ty });
    }

    fn emit_invalid_binary(&mut self, op: BinOp, lhs: Ty, rhs: Ty, span: Span) {
        self.diagnostics
            .push(TypeError::InvalidBinaryOperands { span, op, lhs, rhs });
    }

    fn emit_mismatched_binary(&mut self, op: BinOp, lhs: Ty, rhs: Ty, span: Span) {
        self.diagnostics
            .push(TypeError::MismatchedBinaryOperands { span, op, lhs, rhs });
    }

    fn emit_type_mismatch(&mut self, found: Ty, expected: Ty, span: Span) {
        self.diagnostics
            .push(TypeError::TypeMismatch { span, found, expected });
    }

    fn emit_argument_mismatch(&mut self, index: usize, found: Ty, expected: Ty, call: Span) {
        self.diagnostics.push(TypeError::ArgumentTypeMismatch {
            call,
            index,
            found,
            expected,
        });
    }

    fn emit_not_callable(&mut self, ty: Ty, span: Span) {
        self.diagnostics.push(TypeError::NotCallable { span, ty });
    }

    /// Checks that `found` coerces to `expected` and returns the type to keep
    /// inferring with (the established side).
    ///
    /// `Never` is the bottom type: a `Never` value may be used where any type is
    /// expected, but no other type may be used where `Never` is expected.
    fn check_type(&mut self, found: Ty, expected: Ty, span: Span) -> Ty {
        if !fits(&found, &expected) {
            self.emit_type_mismatch(found, expected, span);
            return Ty::Unknown;
        }

        if matches!(found, Ty::Never | Ty::Unknown) {
            expected
        } else {
            found
        }
    }

    fn check_function(
        &mut self,
        exprs: &mut IndexVec<hir::ExprId, hir::Expr>,
        func: &ast::Function,
        id: FnId,
    ) -> hir::Block {
        func.args.iter().for_each(|(name, ty)| {
            let id = self.resolutions.vars[&name.id()];
            let ty = self.resolutions.tys[&ty.id()].clone();
            self.env.insert(id.unwrap_left(), ty);
        });
        let expected = self.definitions.fns(id).ret.clone();
        let (block, inferred) = self.check_block(exprs, &func.body, &expected);
        self.check_type(inferred, expected, func.body.span());
        block
    }

    fn check_block(
        &mut self,
        exprs: &mut IndexVec<hir::ExprId, hir::Expr>,
        block: &ast::Block,
        ret_ty: &Ty,
    ) -> (hir::Block, Ty) {
        let mut stmts = Vec::with_capacity(block.stmts.len());
        // a block diverges if any of its statements does not fall through
        let mut diverges = false;
        for stmt in &block.stmts {
            let (stmt, ty) = self.check_stmt(exprs, stmt, ret_ty);
            diverges |= matches!(ty, Ty::Never);
            stmts.push(stmt);
        }

        let tail = block
            .tail
            .as_ref()
            .map(|expr| self.check_expr(exprs, expr, ret_ty));

        let ty = if diverges {
            Ty::Never
        } else if let Some((_, ty)) = &tail {
            ty.clone()
        } else {
            Ty::Unit
        };

        (
            hir::Block {
                stmts,
                tail: tail.map(|(id, _)| id),
            },
            ty,
        )
    }

    /// Checks a single statement, returning its hir node together with its
    /// type. The type is only meaningful for its divergence: `Ty::Never` means
    /// control does not fall through, everything else is normalised to
    /// `Ty::Unit`.
    fn check_stmt(
        &mut self,
        exprs: &mut IndexVec<hir::ExprId, hir::Expr>,
        stmt: &ast::Stmt,
        ret_ty: &Ty,
    ) -> (hir::Stmt, Ty) {
        match stmt {
            ast::Stmt::Expr(expr) => {
                let (expr, ty) = self.check_expr(exprs, expr, ret_ty);
                let diverges = matches!(ty, Ty::Never);
                (
                    hir::Stmt::Expr(expr),
                    if diverges { Ty::Never } else { Ty::Unit },
                )
            }
            ast::Stmt::Let { name, ty, value } => {
                let id = self
                    .resolutions
                    .vars
                    .get(&name.id())
                    .copied()
                    .expect("variable not found, something went wrong")
                    .unwrap_left();

                let value_span = value.span();
                let (expr, infered_ty) = self.check_expr(exprs, value, ret_ty);
                let diverges = matches!(infered_ty, Ty::Never);

                let ty = match ty {
                    Some(ty) => {
                        let expected_ty = self
                            .resolutions
                            .tys
                            .get(&ty.id())
                            .cloned()
                            .expect("type not found, something went wrong");

                        self.check_type(infered_ty, expected_ty, value_span)
                    }
                    None => infered_ty,
                };

                self.env.insert(id, ty.clone());

                (
                    hir::Stmt::Let {
                        name: id,
                        ty,
                        value: expr,
                    },
                    if diverges { Ty::Never } else { Ty::Unit },
                )
            }
            ast::Stmt::Assign { place, value } => {
                let value_span = value.span();
                let (value, value_ty) = self.check_expr(exprs, value, ret_ty);
                let (place, place_ty) = self.check_expr(exprs, place, ret_ty);
                let diverges = matches!(value_ty, Ty::Never) || matches!(place_ty, Ty::Never);
                // the value must fit the place it is assigned to
                self.check_type(value_ty, place_ty, value_span);
                (
                    hir::Stmt::Assign { place, value },
                    if diverges { Ty::Never } else { Ty::Unit },
                )
            }
            // a `return` never falls through, so its type is `Never`
            ast::Stmt::Return { value } => {
                let value = value.as_ref().map(|value| {
                    let span = value.span();
                    let (expr, ty) = self.check_expr(exprs, value, ret_ty);
                    self.check_type(ty, ret_ty.clone(), span);
                    expr
                });
                (hir::Stmt::Return { value }, Ty::Never)
            }
            ast::Stmt::Loop { body } => {
                let span = body.span();
                // a loop only falls through if it can be exited with a `break`;
                // otherwise it is an infinite loop of type `Never`
                let can_break = block_breaks(body);
                let (body, ty) = self.check_block(exprs, body, ret_ty);
                self.check_type(ty, Ty::Unit, span);
                (
                    hir::Stmt::Loop { body },
                    if can_break { Ty::Unit } else { Ty::Never },
                )
            }
            ast::Stmt::Break { value } => {
                if let Some(_value) = value {
                    todo!("breaking out of loops with a value is not yet supported");
                }
                (hir::Stmt::Break { value: None }, Ty::Never)
            }
            ast::Stmt::Continue => (hir::Stmt::Continue, Ty::Never),
            ast::Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                let cond_span = cond.span();
                let then_span = then_block.span();

                let (cond, cond_ty) = self.check_expr(exprs, cond, ret_ty);
                let cond_diverges = matches!(cond_ty, Ty::Never);
                self.check_type(cond_ty, Ty::Builtin(BuiltinTy::Bool), cond_span);

                let (then_block, then_ty) = self.check_block(exprs, then_block, ret_ty);
                let then_diverges = matches!(then_ty, Ty::Never);
                self.check_type(then_ty, Ty::Unit, then_span);

                let (else_block, else_diverges) = match else_block {
                    Some(block) => {
                        let span = block.span();
                        let (block, ty) = self.check_block(exprs, block, ret_ty);
                        let diverges = matches!(ty, Ty::Never);
                        self.check_type(ty, Ty::Unit, span);
                        (Some(block), diverges)
                    }
                    None => (None, false),
                };

                // an `if` diverges only if its condition never yields or if
                // both branches always diverge
                let diverges =
                    cond_diverges || (else_block.is_some() && then_diverges && else_diverges);

                (
                    hir::Stmt::If {
                        cond,
                        then_block,
                        else_block,
                    },
                    if diverges { Ty::Never } else { Ty::Unit },
                )
            }
        }
    }

    fn check_expr(
        &mut self,
        exprs: &mut IndexVec<hir::ExprId, hir::Expr>,
        expr: &ast::RExpr,
        ret_ty: &Ty,
    ) -> (hir::ExprId, Ty) {
        match &***expr {
            ast::Expr::Literal(literal) => {
                let (literal, ty) = self.check_literal(literal);
                let id = exprs.push(hir::Expr::Literal(literal));
                (id, ty)
            }
            ast::Expr::Variable(node) => match self.resolutions.vars.get(&node.id()).unwrap() {
                Either::Left(var) => {
                    let ty = self.env[var].clone();
                    (exprs.push(hir::Expr::Variable(*var)), ty)
                }
                Either::Right(func) => {
                    let ty = self.definitions.fns(*func).signature();
                    (exprs.push(hir::Expr::Function(*func)), ty)
                }
            },
            ast::Expr::Grouping(node) => self.check_expr(exprs, node, ret_ty),
            ast::Expr::Binary { lhs, rhs, op } => {
                let (lhs_expr, lhs_ty) = self.check_expr(exprs, lhs, ret_ty);
                let (rhs_expr, rhs_ty) = self.check_expr(exprs, rhs, ret_ty);
                let ty = self.infer_binary(lhs_ty, rhs_ty, *op, expr.span());
                (
                    exprs.push(hir::Expr::Binary {
                        lhs: lhs_expr,
                        rhs: rhs_expr,
                        op: *op,
                    }),
                    ty,
                )
            }
            ast::Expr::Unary { rhs, op } => {
                let (rhs, ty) = self.check_expr(exprs, rhs, ret_ty);
                let ty = self.infer_unary(ty, *op, expr.span());
                (exprs.push(hir::Expr::Unary { rhs, op: *op }), ty)
            }
            ast::Expr::Field { lhs, name } => {
                let (lhs, ty) = self.check_expr(exprs, lhs, ret_ty);
                let (index, field_ty) = self.check_field(ty, name);
                (exprs.push(hir::Expr::Field { lhs, index }), field_ty)
            }
            ast::Expr::Call { lhs, args } => {
                let (func, ty) = self.check_expr(exprs, lhs, ret_ty);
                let (args, tys) = args
                    .iter()
                    .map(|arg| self.check_expr(exprs, arg, ret_ty))
                    .unzip::<_, _, Vec<_>, Vec<_>>();
                let ret = self.check_call(ty, &tys, expr.span());
                (exprs.push(hir::Expr::Call { lhs: func, args }), ret)
            }
            ast::Expr::MethodCall { .. } => {
                todo!("method call is not yet supported")
            }
            ast::Expr::Index { lhs, index } => {
                // note: indexing is only implemented for some primitive types, currently.
                // the idea is to eventually, once methods come into play and i settle on an
                // interface design, completly desugar to method calls
                let (lhs, lhs_ty) = self.check_expr(exprs, lhs, ret_ty);
                let (index, index_ty) = self.check_expr(exprs, index, ret_ty);
                let ret = self.check_index(lhs_ty, index_ty, expr.span());
                (exprs.push(hir::Expr::Index { lhs, index }), ret)
            }
            ast::Expr::Ref(node) => {
                let (expr, ty) = self.check_expr(exprs, node, ret_ty);
                // referencing an expression that never yields also diverges
                let ty = if matches!(ty, Ty::Never) {
                    Ty::Never
                } else {
                    Ty::Pointer(Box::new(ty))
                };
                (exprs.push(hir::Expr::Ref(expr)), ty)
            }
            ast::Expr::Deref(node) => {
                let (reference, ty) = self.check_expr(exprs, node, ret_ty);
                let ret = self.check_deref(ty, expr.span());
                (exprs.push(hir::Expr::Deref(reference)), ret)
            }
            ast::Expr::Block(block) => {
                let (block, ty) = self.check_block(exprs, block, ret_ty);
                (exprs.push(hir::Expr::Block(block)), ty)
            }
        }
    }

    fn check_deref(&mut self, ty: Ty, span: Span) -> Ty {
        match ty {
            Ty::Pointer(ty) => *ty,
            // the operand never yields, so the dereference never yields either
            Ty::Never => Ty::Never,
            Ty::Unknown => ty,
            _ => {
                self.diagnostics.push(TypeError::InvalidDeref { span, ty });
                Ty::Unknown
            }
        }
    }

    fn check_index(&mut self, lhs: Ty, index: Ty, span: Span) -> Ty {
        // indexing into, or with, an expression that never yields diverges
        if matches!(lhs, Ty::Never) || matches!(index, Ty::Never) {
            return Ty::Never;
        }

        match lhs {
            Ty::Pointer(ref ty) | Ty::Array(ref ty) => {
                let ret = (**ty).clone();
                match index {
                    Ty::Builtin(
                        BuiltinTy::U8 | BuiltinTy::U16 | BuiltinTy::U32 | BuiltinTy::U64,
                    )
                    | Ty::Unknown => {}
                    _ => {
                        self.diagnostics.push(TypeError::InvalidIndexType {
                            ty: lhs,
                            span,
                            index_ty: index,
                        });
                    }
                }
                ret
            }
            _ => {
                self.diagnostics
                    .push(TypeError::NonIndexableType { span, ty: lhs });
                Ty::Unknown
            }
        }
    }

    fn check_call(&mut self, func: Ty, args: &[Ty], call_span: Span) -> Ty {
        match func {
            Ty::Pointer(ty) => self.check_call(*ty, args, call_span),
            // calling an expression that never yields diverges
            Ty::Never => Ty::Never,
            Ty::Fn(ref ret, ref params) => {
                // get the return type
                let ret = (**ret).clone();
                // check if the arguments fit their parameters
                args.iter()
                    .zip(params.iter())
                    .enumerate()
                    .for_each(|(index, (arg, expected))| {
                        if !fits(arg, expected) {
                            self.emit_argument_mismatch(
                                index,
                                arg.clone(),
                                expected.clone(),
                                call_span,
                            );
                        }
                    });

                // check if the count/arity matches
                if params.len() != args.len() {
                    self.diagnostics.push(TypeError::ArgumentCountMismatch {
                        call: call_span,
                        func,
                        count: args.len(),
                    });
                }

                ret
            }
            _ => {
                self.emit_not_callable(func, call_span);
                Ty::Unknown
            }
        }
    }

    fn check_field(&mut self, ty: Ty, field: &Ident) -> (usize, Ty) {
        match ty {
            Ty::Pointer(ty) => self.check_field((*ty).clone(), field),
            // the receiver never yields, so the field access never yields either
            Ty::Never => (0, Ty::Never),
            Ty::Custom(id) => {
                let def = self.definitions.tys(id);
                match def {
                    TyDef::Struct { fields } => {
                        match fields.iter().enumerate().find_map(|(idx, (name, ty))| {
                            if name.eq(field) {
                                Some((idx, ty))
                            } else {
                                None
                            }
                        }) {
                            Some((idx, ty)) => (idx, ty.clone()),
                            None => {
                                let len = fields.len();
                                self.emit_unknown_field(ty.clone(), field);
                                (len, Ty::Unknown)
                            }
                        }
                    }
                    TyDef::Alias(ty) => self.check_field(ty.clone(), field),
                    TyDef::Enum { .. } => {
                        self.emit_unknown_field(ty.clone(), field);
                        (0, Ty::Unknown)
                    }
                }
            }
            _ => {
                self.emit_unknown_field(ty.clone(), field);
                (0, Ty::Unknown)
            }
        }
    }

    fn check_literal(&mut self, literal: &token::Literal) -> (hir::Literal, Ty) {
        match literal {
            token::Literal::Int(i) => (hir::Literal::Int(*i), Ty::Builtin(BuiltinTy::I32)),
            token::Literal::Float(f) => (hir::Literal::Float(*f), Ty::Builtin(BuiltinTy::F32)),
            token::Literal::Bool(b) => (hir::Literal::Bool(*b), Ty::Builtin(BuiltinTy::Bool)),
            token::Literal::String(cow) => (
                hir::Literal::String(cow.clone().into_owned()),
                Ty::Builtin(BuiltinTy::String),
            ),
            token::Literal::Char(c) => (hir::Literal::Char(*c), Ty::Builtin(BuiltinTy::Char)),
        }
    }

    // we do not yet support user defined operator methods
    fn infer_unary(&mut self, ty: Ty, op: UnOp, span: Span) -> Ty {
        match op {
            // `!` is logical negation for booleans and bitwise negation for
            // integers; in both cases the operand and result types are equal.
            UnOp::Not if matches!(ty, Ty::Builtin(BuiltinTy::Bool)) || is_integer(&ty) => ty,
            // `-` is defined for signed integers and floats and preserves the type.
            UnOp::Neg if is_signed_integer(&ty) || is_float(&ty) => ty,
            // an operand that never yields makes the whole expression diverge
            _ if matches!(ty, Ty::Never) => Ty::Never,
            // the operand already failed to be inferred, so don't add noise.
            _ if matches!(ty, Ty::Unknown) => Ty::Unknown,
            _ => {
                self.emit_invalid_unary(op, ty, span);
                Ty::Unknown
            }
        }
    }

    // we do not yet support user defined operator methods
    fn infer_binary(&mut self, lhs: Ty, rhs: Ty, op: BinOp, span: Span) -> Ty {
        use BinOp::*;

        // if either operand never yields, the whole expression diverges
        if matches!(lhs, Ty::Never) || matches!(rhs, Ty::Never) {
            return Ty::Never;
        }

        // if either operand failed to be inferred there is nothing meaningful
        // to check, and reporting here would only produce cascading errors.
        if matches!(lhs, Ty::Unknown) || matches!(rhs, Ty::Unknown) {
            return Ty::Unknown;
        }

        let bool_ty = Ty::Builtin(BuiltinTy::Bool);

        match op {
            // arithmetic operators require two operands of the same numeric type
            Add | Sub | Mul | Div | Mod => {
                if !is_numeric(&lhs) || !is_numeric(&rhs) {
                    self.emit_invalid_binary(op, lhs, rhs, span);
                    Ty::Unknown
                } else if lhs != rhs {
                    self.emit_mismatched_binary(op, lhs, rhs, span);
                    Ty::Unknown
                } else {
                    lhs
                }
            }
            // equality is defined for builtins, pointers and unit, and always
            // yields a boolean
            Eq | Ne => {
                if !is_equatable(&lhs) || !is_equatable(&rhs) {
                    self.emit_invalid_binary(op, lhs, rhs, span);
                    Ty::Unknown
                } else if lhs != rhs {
                    self.emit_mismatched_binary(op, lhs, rhs, span);
                    Ty::Unknown
                } else {
                    bool_ty
                }
            }
            // ordering is defined for numbers, chars and strings
            Lt | Le | Gt | Ge => {
                if !is_ordered(&lhs) || !is_ordered(&rhs) {
                    self.emit_invalid_binary(op, lhs, rhs, span);
                    Ty::Unknown
                } else if lhs != rhs {
                    self.emit_mismatched_binary(op, lhs, rhs, span);
                    Ty::Unknown
                } else {
                    bool_ty
                }
            }
            // logical operators only accept booleans
            And | Or => {
                if matches!(lhs, Ty::Builtin(BuiltinTy::Bool))
                    && matches!(rhs, Ty::Builtin(BuiltinTy::Bool))
                {
                    bool_ty
                } else {
                    self.emit_invalid_binary(op, lhs, rhs, span);
                    Ty::Unknown
                }
            }
        }
    }
}

/// Whether `found` coerces to `expected`. `Never` is the bottom type and
/// therefore fits any expected type, and `Unknown` is treated as a wildcard so
/// that already-reported errors do not cascade.
fn fits(found: &Ty, expected: &Ty) -> bool {
    matches!(found, Ty::Never | Ty::Unknown)
        || matches!(expected, Ty::Unknown)
        || found == expected
}

fn is_integer(ty: &Ty) -> bool {
    matches!(
        ty,
        Ty::Builtin(
            BuiltinTy::I8
                | BuiltinTy::I16
                | BuiltinTy::I32
                | BuiltinTy::I64
                | BuiltinTy::U8
                | BuiltinTy::U16
                | BuiltinTy::U32
                | BuiltinTy::U64
        )
    )
}

fn is_signed_integer(ty: &Ty) -> bool {
    matches!(
        ty,
        Ty::Builtin(BuiltinTy::I8 | BuiltinTy::I16 | BuiltinTy::I32 | BuiltinTy::I64)
    )
}

fn is_float(ty: &Ty) -> bool {
    matches!(ty, Ty::Builtin(BuiltinTy::F32 | BuiltinTy::F64))
}

fn is_numeric(ty: &Ty) -> bool {
    is_integer(ty) || is_float(ty)
}

fn is_equatable(ty: &Ty) -> bool {
    matches!(ty, Ty::Builtin(_) | Ty::Pointer(_) | Ty::Unit)
}

fn is_ordered(ty: &Ty) -> bool {
    is_numeric(ty) || matches!(ty, Ty::Builtin(BuiltinTy::Char | BuiltinTy::String))
}

/// Whether a loop body contains a `break` that targets the loop it belongs to.
///
/// Nested loops are skipped because their `break`s target themselves. `break`
/// can also appear inside block expressions, so the scan descends into
/// expressions as well.
fn block_breaks(block: &ast::RBlock) -> bool {
    block.stmts.iter().any(|stmt| stmt_breaks(stmt))
        || block.tail.as_ref().is_some_and(|expr| expr_breaks(expr))
}

fn stmt_breaks(stmt: &ast::Stmt) -> bool {
    match stmt {
        ast::Stmt::Break { .. } => true,
        ast::Stmt::Loop { .. } => false,
        ast::Stmt::Continue => false,
        ast::Stmt::Expr(expr) => expr_breaks(expr),
        ast::Stmt::Let { value, .. } => expr_breaks(value),
        ast::Stmt::Assign { place, value } => expr_breaks(place) || expr_breaks(value),
        ast::Stmt::Return { value } => value.as_ref().is_some_and(|value| expr_breaks(value)),
        ast::Stmt::If {
            cond,
            then_block,
            else_block,
        } => {
            expr_breaks(cond)
                || block_breaks(then_block)
                || else_block.as_ref().is_some_and(|block| block_breaks(block))
        }
    }
}

fn expr_breaks(expr: &ast::RExpr) -> bool {
    match &***expr {
        ast::Expr::Block(block) => block_breaks(block),
        ast::Expr::Grouping(node) => expr_breaks(node),
        ast::Expr::Binary { lhs, rhs, .. } => expr_breaks(lhs) || expr_breaks(rhs),
        ast::Expr::Unary { rhs, .. } => expr_breaks(rhs),
        ast::Expr::Field { lhs, .. } => expr_breaks(lhs),
        ast::Expr::Call { lhs, args } => expr_breaks(lhs) || args.iter().any(|arg| expr_breaks(arg)),
        ast::Expr::MethodCall { lhs, args, .. } => {
            expr_breaks(lhs) || args.iter().any(|arg| expr_breaks(arg))
        }
        ast::Expr::Index { lhs, index } => expr_breaks(lhs) || expr_breaks(index),
        ast::Expr::Ref(node) | ast::Expr::Deref(node) => expr_breaks(node),
        ast::Expr::Literal(_) | ast::Expr::Variable(_) => false,
    }
}
