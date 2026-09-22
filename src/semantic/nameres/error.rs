use crate::semantic::ast::Symbol;
use crate::semantic::node::Span;

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
        symbol: crate::semantic::node::Node<string_interner::symbol::SymbolU32>,
        original: chumsky::prelude::SimpleSpan,
        duplicate: chumsky::prelude::SimpleSpan,
    },
    DuplicateFieldDefinition {
        symbol: crate::semantic::node::Node<string_interner::symbol::SymbolU32>,
        original: chumsky::prelude::SimpleSpan,
        duplicate: chumsky::prelude::SimpleSpan,
    },
}
