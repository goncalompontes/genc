use std::sync::Arc;

pub enum Either<A, B> {
    Left(A),
    Right(B),
}

pub type Tree<Tok> = Arc<TreeData<Tok>>;
pub struct TreeData<Tok> {
    children: Vec<Either<Tree<Tok>, Tok>>,
}

pub struct Token<Kind, Repr> {
    kind: Kind,
    repr: Repr,
}


trait Input {
    type Slice<'src>;
}