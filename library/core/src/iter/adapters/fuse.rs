use crate::intrinsics;
use crate::iter::adapters::SourceIter;
use crate::iter::adapters::zip::try_get_unchecked;
use crate::iter::{
    FusedIterator, TrustedFused, TrustedLen, TrustedRandomAccess, TrustedRandomAccessNoCoerce,
};
use crate::ops::Try;

/// An iterator that yields `None` forever after the underlying iterator
/// yields `None` once.
///
/// This `struct` is created by [`Iterator::fuse`]. See its documentation
/// for more.
#[derive(Clone, Debug)]
#[must_use = "iterators are lazy and do nothing unless consumed"]
#[stable(feature = "rust1", since = "1.0.0")]
pub struct Fuse<I> {
    // NOTE: for `I: FusedIterator`, we never bother setting `None`, but
    // we still have to be prepared for that state due to variance.
    // See rust-lang/rust#85863
    iter: Option<I>,
}
impl<I> Fuse<I> {
    pub(in crate::iter) fn new(iter: I) -> Fuse<I> {
        Fuse { iter: Some(iter) }
    }

    pub(crate) fn into_inner(self) -> Option<I> {
        self.iter
    }

    const IS_FUSED: bool = crate::intrinsics::vtable_for::<I, dyn FusedIterator>().is_some();
}

#[stable(feature = "fused", since = "1.26.0")]
impl<I> FusedIterator for Fuse<I> where I: Iterator {}

#[unstable(issue = "none", feature = "trusted_fused")]
unsafe impl<I> TrustedFused for Fuse<I> where I: TrustedFused {}

// Any specialized implementation here is made internal
// to avoid exposing default fns outside this trait.
#[stable(feature = "rust1", since = "1.0.0")]
impl<I> Iterator for Fuse<I>
where
    I: Iterator,
{
    type Item = <I as Iterator>::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        and_then_or_clear(&mut self.iter, Iterator::next)
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<I::Item> {
        and_then_or_clear(&mut self.iter, |iter| iter.nth(n))
    }

    #[inline]
    fn last(self) -> Option<Self::Item> {
        match self.iter {
            Some(iter) => iter.last(),
            None => None,
        }
    }

    #[inline]
    fn count(self) -> usize {
        match self.iter {
            Some(iter) => iter.count(),
            None => 0,
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.iter {
            Some(ref iter) => iter.size_hint(),
            None => (0, Some(0)),
        }
    }

    #[inline]
    fn try_fold<Acc, Fold, R>(&mut self, mut acc: Acc, fold: Fold) -> R
    where
        Self: Sized,
        Fold: FnMut(Acc, Self::Item) -> R,
        R: Try<Output = Acc>,
    {
        if let Some(ref mut iter) = self.iter {
            acc = iter.try_fold(acc, fold)?;
            if !Self::IS_FUSED {
                self.iter = None;
            }
        }
        try { acc }
    }

    #[inline]
    fn fold<Acc, Fold>(self, mut acc: Acc, fold: Fold) -> Acc
    where
        Fold: FnMut(Acc, Self::Item) -> Acc,
    {
        if let Some(iter) = self.iter {
            acc = iter.fold(acc, fold);
        }
        acc
    }

    #[inline]
    fn find<P>(&mut self, predicate: P) -> Option<Self::Item>
    where
        P: FnMut(&Self::Item) -> bool,
    {
        and_then_or_clear(&mut self.iter, |iter| iter.find(predicate))
    }

    #[inline]
    unsafe fn __iterator_get_unchecked(&mut self, idx: usize) -> Self::Item
    where
        Self: TrustedRandomAccessNoCoerce,
    {
        match self.iter {
            // SAFETY: the caller must uphold the contract for
            // `Iterator::__iterator_get_unchecked`.
            Some(ref mut iter) => unsafe { try_get_unchecked(iter, idx) },
            // SAFETY: the caller asserts there is an item at `i`, so we're not exhausted.
            None => unsafe { intrinsics::unreachable() },
        }
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<I> DoubleEndedIterator for Fuse<I>
where
    I: DoubleEndedIterator,
{
    #[inline]
    fn next_back(&mut self) -> Option<<I as Iterator>::Item> {
        and_then_or_clear(&mut self.iter, |iter| iter.next_back())
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<<I as Iterator>::Item> {
        and_then_or_clear(&mut self.iter, |iter| iter.nth_back(n))
    }

    #[inline]
    fn try_rfold<Acc, Fold, R>(&mut self, mut acc: Acc, fold: Fold) -> R
    where
        Self: Sized,
        Fold: FnMut(Acc, Self::Item) -> R,
        R: Try<Output = Acc>,
    {
        if let Some(ref mut iter) = self.iter {
            acc = iter.try_rfold(acc, fold)?;
            if !Self::IS_FUSED {
                self.iter = None;
            }
        }
        try { acc }
    }

    #[inline]
    fn rfold<Acc, Fold>(self, mut acc: Acc, fold: Fold) -> Acc
    where
        Fold: FnMut(Acc, Self::Item) -> Acc,
    {
        if let Some(iter) = self.iter {
            acc = iter.rfold(acc, fold);
        }
        acc
    }

    #[inline]
    fn rfind<P>(&mut self, predicate: P) -> Option<Self::Item>
    where
        P: FnMut(&Self::Item) -> bool,
    {
        and_then_or_clear(&mut self.iter, |iter| iter.rfind(predicate))
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<I> ExactSizeIterator for Fuse<I>
where
    I: ExactSizeIterator,
{
    fn len(&self) -> usize {
        match self.iter {
            Some(ref iter) => iter.len(),
            None => 0,
        }
    }

    fn is_empty(&self) -> bool {
        match self.iter {
            Some(ref iter) => iter.is_empty(),
            None => true,
        }
    }
}

#[stable(feature = "default_iters", since = "1.70.0")]
impl<I: Default> Default for Fuse<I> {
    /// Creates a `Fuse` iterator from the default value of `I`.
    ///
    /// ```
    /// # use core::slice;
    /// # use std::iter::Fuse;
    /// let iter: Fuse<slice::Iter<'_, u8>> = Default::default();
    /// assert_eq!(iter.len(), 0);
    /// ```
    ///
    /// This is equivalent to `I::default().fuse()`[^fuse_note]; e.g. if
    /// `I::default()` is not an empty iterator, then this will not be
    /// an empty iterator.
    ///
    /// ```
    /// # use std::iter::Fuse;
    /// #[derive(Default)]
    /// struct Fourever;
    ///
    /// impl Iterator for Fourever {
    ///     type Item = u32;
    ///     fn next(&mut self) -> Option<u32> {
    ///         Some(4)
    ///     }
    /// }
    ///
    /// let mut iter: Fuse<Fourever> = Default::default();
    /// assert_eq!(iter.next(), Some(4));
    /// ```
    ///
    /// [^fuse_note]: if `I` does not override `Iterator::fuse`'s default implementation
    fn default() -> Self {
        Fuse { iter: Some(I::default()) }
    }
}

#[unstable(feature = "trusted_len", issue = "37572")]
// SAFETY: `TrustedLen` requires that an accurate length is reported via `size_hint()`. As `Fuse`
// is just forwarding this to the wrapped iterator `I` this property is preserved and it is safe to
// implement `TrustedLen` here.
unsafe impl<I> TrustedLen for Fuse<I> where I: TrustedLen {}

#[doc(hidden)]
#[unstable(feature = "trusted_random_access", issue = "none")]
// SAFETY: `TrustedRandomAccess` requires that `size_hint()` must be exact and cheap to call, and
// `Iterator::__iterator_get_unchecked()` must be implemented accordingly.
//
// This is safe to implement as `Fuse` is just forwarding these to the wrapped iterator `I`, which
// preserves these properties.
unsafe impl<I> TrustedRandomAccess for Fuse<I> where I: TrustedRandomAccess {}

#[doc(hidden)]
#[unstable(feature = "trusted_random_access", issue = "none")]
unsafe impl<I> TrustedRandomAccessNoCoerce for Fuse<I>
where
    I: TrustedRandomAccessNoCoerce,
{
    const MAY_HAVE_SIDE_EFFECT: bool = I::MAY_HAVE_SIDE_EFFECT;
}

// This is used by Flatten's SourceIter impl
#[unstable(issue = "none", feature = "inplace_iteration")]
unsafe impl<I> SourceIter for Fuse<I>
where
    I: SourceIter + TrustedFused,
{
    type Source = I::Source;

    #[inline]
    unsafe fn as_inner(&mut self) -> &mut I::Source {
        // SAFETY: unsafe function forwarding to unsafe function with the same requirements.
        // TrustedFused guarantees that we'll never encounter a case where `self.iter` would
        // be set to None.
        unsafe { SourceIter::as_inner(self.iter.as_mut().unwrap_unchecked()) }
    }
}

#[inline]
fn and_then_or_clear<T, U>(opt: &mut Option<T>, f: impl FnOnce(&mut T) -> Option<U>) -> Option<U> {
    let x = f(opt.as_mut()?);
    // If the type implements FusedIterator, we don't need to fuse at runtime
    if const { crate::intrinsics::vtable_for::<T, dyn FusedIterator>().is_none() } && x.is_none() {
        *opt = None;
    }
    x
}
