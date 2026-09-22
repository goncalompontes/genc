use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use chumsky::span::SimpleSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(usize);

impl FileId {
    pub fn into_inner(&self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId {
    node_id: usize,
    file_id: FileId,
}

impl NodeId {
    pub fn into_inner(&self) -> (usize, usize) {
        (self.node_id, self.file_id.into_inner())
    }

    pub fn file_id(&self) -> &FileId {
        &self.file_id
    }

    pub fn node_id(&self) -> usize {
        self.node_id
    }

    pub fn new(node_id: usize, file_id: FileId) -> Self {
        Self { node_id, file_id }
    }

    pub fn cast<T>(&self) -> Id<T> {
        Id {
            id: self.clone(),
            _marker: PhantomData,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Id<T> {
    id: NodeId,
    _marker: PhantomData<T>,
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T> Copy for Id<T> {}

impl<T> Id<T> {
    pub fn into_inner(&self) -> (usize, usize) {
        self.id.into_inner()
    }

    pub fn file_id(&self) -> &FileId {
        self.id.file_id()
    }

    pub fn node_id(&self) -> usize {
        self.id.node_id()
    }

    pub fn new(node_id: usize, file_id: FileId) -> Id<T> {
        Id {
            id: NodeId::new(node_id, file_id),
            _marker: PhantomData,
        }
    }
}

pub type Span = SimpleSpan;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Spanned<T> {
    pub span: Span,
    pub value: T,
}

impl<T> Spanned<T> {
    pub fn new(span: Span, value: T) -> Self {
        Self { span, value }
    }
}

impl<T> Spanned<T> {
    pub fn span(&self) -> Span {
        self.span
    }

    pub fn value(&self) -> &T {
        &self.value
    }
}

impl<T> Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value()
    }
}
impl<T> DerefMut for Spanned<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}
