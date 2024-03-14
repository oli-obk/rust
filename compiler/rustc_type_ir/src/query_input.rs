use crate::{fold::TypeFoldable, Interner};

pub trait QueryInput<I: Interner>: TypeFoldable<I> {
    fn defining_opaque_types(&self) -> I::DefiningOpaqueTypes;
}

impl<I: Interner, A: TypeFoldable<I>, B: TypeFoldable<I>> QueryInput<I>
    for (I::DefiningOpaqueTypes, A, B)
{
    fn defining_opaque_types(&self) -> <I as Interner>::DefiningOpaqueTypes {
        self.0
    }
}
