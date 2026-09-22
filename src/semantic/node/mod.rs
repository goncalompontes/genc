use std::ops::{Deref, DerefMut};

mod id;
mod span;

pub use id::{FileId, Id, NodeId};
pub use span::{Span, Spanned};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Node<T> {
    value: T,
    span: Span,
    id: Id<T>,
}

impl<T> Node<T> {
    pub fn new(value: T, span: Span, id: Id<T>) -> Self {
        Self { value, span, id }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn value_mut(&mut self) -> &mut T {
        &mut self.value
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn id(&self) -> Id<T> {
        self.id
    }
}

impl<T> DerefMut for Node<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.value_mut()
    }
}

impl<T> Deref for Node<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value()
    }
}

pub type Ref<'ast, T> = Node<&'ast T>;
pub type Mut<'ast, T> = Node<&'ast mut T>;
