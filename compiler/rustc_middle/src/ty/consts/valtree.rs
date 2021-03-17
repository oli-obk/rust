use crate::ty::TyCtxt;

use super::ScalarInt;
use rustc_macros::HashStable;

#[derive(Copy, Clone, Debug, Hash, TyEncodable, TyDecodable, Eq, PartialEq, Ord, PartialOrd)]
#[derive(HashStable)]
/// This datastructure is used to represent the value of constants used in the type system.
///
/// We explicitly choose a different datastructure from the way values are processed within
/// CTFE, as in the type system equal values (according to their `PartialEq`) must also have
/// equal representation (`==` on the rustc data structure, e.g. `ValTree`) and vice versa.
/// Since CTFE uses `AllocId` to represent pointers, it often happens that two different
/// `AllocId`s point to equal values. So we may end up with different representations for
/// two constants whose value is `&42`. Furthermore any kind of struct that has padding will
/// have arbitrary values within that padding, even if the values of the struct are the same.
///
/// `ValTree` does not have this problem with representation, as it only contains integers or
/// lists of (nested) `ValTree`.
///
/// Note: References do not create a single-element `Branch`, but instead encode their pointee
/// directly. So the representation of `&&&42` is the same as `42`.
pub enum ValTree<'tcx> {
    /// ZSTs, integers, `bool`, `char` are represented as scalars.
    /// See the `ScalarInt` documentation for how `ScalarInt` guarantees that equal values
    /// of these types have the same representation.
    Leaf(ScalarInt),
    /// The fields of any kind of aggregate. Structs, tuples and arrays are represented by
    /// listing their fields' values in order.
    /// Enums are represented by storing their discriminant as a field, followed by all
    /// the fields of the variant.
    ///
    /// `&str` and `&[T]` are encoded as if they were `&[T;N]`. So there is no wide pointer
    /// or metadata encoded, instead the length is taken directly from the number of elements
    /// in the branch.
    Branch(&'tcx [ValTree<'tcx>]),
}

impl ValTree<'tcx> {
    pub fn zst() -> Self {
        Self::Branch(&[])
    }
    pub fn unwrap_leaf(self) -> ScalarInt {
        match self {
            Self::Leaf(s) => s,
            Self::Branch(branch) => bug!("expected leaf, got {:?}", branch),
        }
    }
    pub fn unwrap_branch(self) -> &'tcx [Self] {
        match self {
            Self::Leaf(s) => bug!("expected branch, got {:?}", s),
            Self::Branch(branch) => branch,
        }
    }
    #[inline]
    pub fn try_to_scalar_int(self) -> Option<ScalarInt> {
        match self {
            Self::Leaf(s) => Some(s),
            Self::Branch(_) => None,
        }
    }

    #[inline]
    pub fn try_to_machine_usize(self, tcx: TyCtxt<'tcx>) -> Option<u64> {
        self.try_to_scalar_int()?.try_to_machine_usize(tcx).ok()
    }
}
