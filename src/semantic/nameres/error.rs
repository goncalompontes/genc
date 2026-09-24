use crate::semantic::ast::Symbol;
use crate::semantic::node::Span;

pub enum ResolutionError {
    DuplicateTypeDefinition {
        symbol: Symbol,
        original: Option<Span>,
        duplicate: Span,
    },
    DuplicateFunctionDefinition {
        symbol: Symbol,
        original: Span,
        duplicate: Span,
    },
    DuplicateVariantDefinition {
        symbol: Symbol,
        original: Span,
        duplicate: Span,
    },
    DuplicateFieldDefinition {
        symbol: Symbol,
        original: Span,
        duplicate: Span,
    },
    UnknownType {
        span: Span,
        name: Symbol,
    },
    DuplicateArgumentDefinition {
        symbol: Symbol,
        original: Span,
        duplicate: Span,
    },
    UnknownName { span: chumsky::prelude::SimpleSpan, name: string_interner::symbol::SymbolU32 },
}
