// Now that we have a fully resolved AST, we can start type checking.
// Since we don't yet support generics or any complex type features,
// the type system consists of just simple bidirectional type checking.

use std::ops::Deref;

use index_vec::IndexVec;
use itertools::{Either, Itertools};
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

    fn unify(&mut self, lhs: Ty, rhs: Ty) -> Ty {
        match (lhs, rhs) {
            (Ty::Unknown, rhs) => rhs,
            (lhs, Ty::Unknown) => lhs,
            (lhs, rhs) => {
                if lhs.eq(&rhs) {
                    lhs
                } else {
                    Ty::Unknown
                }
            }
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
        self.unify(expected, inferred);
        block
    }

    fn check_block(
        &mut self,
        exprs: &mut IndexVec<hir::ExprId, hir::Expr>,
        block: &ast::Block,
        ret_ty: &Ty,
    ) -> (hir::Block, Ty) {
        let stmts = block
            .stmts
            .iter()
            .map(|stmt| self.check_stmt(exprs, stmt, ret_ty))
            .collect_vec();
        let tail = block
            .tail
            .as_ref()
            .map(|expr| self.check_expr(exprs, expr, ret_ty));
        let (tail, ty) = if let Some((id, ty)) = tail {
            (Some(id), ty)
        } else {
            (None, Ty::Unit)
        };
        (hir::Block { stmts, tail }, ty)
    }

    fn check_stmt(
        &mut self,
        exprs: &mut IndexVec<hir::ExprId, hir::Expr>,
        stmt: &ast::Stmt,
        ret_ty: &Ty,
    ) -> hir::Stmt {
        match stmt {
            ast::Stmt::Expr(expr) => hir::Stmt::Expr(self.check_expr(exprs, expr, ret_ty).0),
            ast::Stmt::Let { name, ty, value } => {
                let id = self
                    .resolutions
                    .vars
                    .get(&name.id())
                    .copied()
                    .expect("variable not found, something went wrong")
                    .unwrap_left();

                let (expr, infered_ty) = self.check_expr(exprs, value, ret_ty);

                let ty = match ty {
                    Some(ty) => {
                        let expected_ty = self
                            .resolutions
                            .tys
                            .get(&ty.id())
                            .cloned()
                            .expect("type not found, something went wrong");

                        self.unify(infered_ty, expected_ty)
                    }
                    None => infered_ty,
                };

                self.env.insert(id, ty.clone());

                hir::Stmt::Let {
                    name: id,
                    ty,
                    value: expr,
                }
            }
            ast::Stmt::Assign { place, value } => {
                let (value, infered_ty) = self.check_expr(exprs, value, ret_ty);
                let (place, expected_ty) = self.check_expr(exprs, place, ret_ty);
                self.unify(infered_ty, expected_ty);
                hir::Stmt::Assign { place, value }
            }
            // we are not checking that the type being returned here is correct
            // need to pass that information down the context
            ast::Stmt::Return { value } => hir::Stmt::Return {
                value: value.map(|v| {
                    let (expr, ty) = self.check_expr(exprs, &v, ret_ty);
                    self.unify(ty, ret_ty.clone());
                    expr
                }),
            },
            ast::Stmt::Loop { body } => {
                let (body, ty) = self.check_block(exprs, body, ret_ty);
                self.unify(ty, Ty::Unit);
                hir::Stmt::Loop { body }
            }
            ast::Stmt::Break { value } => {
                if let Some(_value) = value {
                    todo!("breaking out of loops with a value is not yet supported");
                }
                hir::Stmt::Break { value: None }
            }
            ast::Stmt::Continue => hir::Stmt::Continue,
            ast::Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                let (cond, cond_ty) = self.check_expr(exprs, cond, ret_ty);
                self.unify(cond_ty, Ty::Builtin(BuiltinTy::Bool));
                let (then_block, then_ty) = self.check_block(exprs, then_block, ret_ty);
                self.unify(then_ty, Ty::Unit);
                let else_block = else_block.map(|block| {
                    let (b, ty) = self.check_block(exprs, &block, ret_ty);
                    self.unify(ty, Ty::Unit);
                    b
                });
                hir::Stmt::If {
                    cond,
                    then_block,
                    else_block,
                }
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
                (exprs.push(hir::Expr::Ref(expr)), Ty::Pointer(Box::new(ty)))
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
            Ty::Unknown => ty,
            _ => {
                self.diagnostics.push(TypeError::InvalidDeref { span, ty });
                Ty::Unknown
            }
        }
    }

    fn check_index(&mut self, lhs: Ty, index: Ty, span: Span) -> Ty {
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
            Ty::Fn(ref ret, ref params) => {
                // get the return type
                let ret = (**ret).clone();
                // check if the types match
                args.iter().zip(params.iter()).for_each(|(arg, expected)| {
                    self.unify(arg.clone(), expected.clone());
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
                todo!("emit error for trying to an uncallable expression")
            }
        }
    }

    fn check_field(&mut self, ty: Ty, field: &Ident) -> (usize, Ty) {
        match ty {
            Ty::Pointer(ty) => self.check_field((*ty).clone(), field),
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
            // the operand already failed to be inferred, so don't add noise.
            _ if matches!(ty, Ty::Unknown | Ty::Never) => Ty::Unknown,
            _ => {
                self.emit_invalid_unary(op, ty, span);
                Ty::Unknown
            }
        }
    }

    // we do not yet support user defined operator methods
    fn infer_binary(&mut self, lhs: Ty, rhs: Ty, op: BinOp, span: Span) -> Ty {
        use BinOp::*;

        // if either operand failed to be inferred there is nothing meaningful
        // to check, and reporting here would only produce cascading errors.
        if matches!(lhs, Ty::Unknown | Ty::Never) || matches!(rhs, Ty::Unknown | Ty::Never) {
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
