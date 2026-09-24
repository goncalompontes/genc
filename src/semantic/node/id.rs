use std::hash::Hash;
use std::marker::PhantomData;

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

#[derive(Debug)]
pub struct Id<T> {
    id: NodeId,
    _marker: PhantomData<T>,
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self._marker.hash(state);
    }
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
