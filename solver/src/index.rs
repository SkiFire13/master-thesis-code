use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};

#[derive(Clone)]
pub struct IndexedVec<I, T> {
    vec: Vec<T>,
    _marker: PhantomData<I>,
}

impl<I, T> Deref for IndexedVec<I, T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.vec
    }
}

impl<I, T> DerefMut for IndexedVec<I, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vec
    }
}

impl<I: AsIndex, T> IndexedVec<I, T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, value: T) -> I {
        let index = I::from_usize(self.len());
        self.vec.push(value);
        index
    }

    pub fn into_enumerate(
        self,
    ) -> impl Iterator<Item = (I, T)> + DoubleEndedIterator + ExactSizeIterator {
        self.vec.into_iter().enumerate().map(|(i, t)| (I::from_usize(i), t))
    }

    pub fn enumerate(
        &self,
    ) -> impl Iterator<Item = (I, &T)> + DoubleEndedIterator + ExactSizeIterator {
        self.vec.iter().enumerate().map(|(i, t)| (I::from_usize(i), t))
    }

    pub fn enumerate_mut(
        &mut self,
    ) -> impl Iterator<Item = (I, &mut T)> + DoubleEndedIterator + ExactSizeIterator {
        self.vec.iter_mut().enumerate().map(|(i, t)| (I::from_usize(i), t))
    }

    pub fn indexes(&self) -> impl Iterator<Item = I> {
        (0..self.len()).map(I::from_usize)
    }

    pub fn last_index(&self) -> Option<I> {
        self.len().checked_sub(1).map(I::from_usize)
    }
}

impl<I, T> Default for IndexedVec<I, T> {
    fn default() -> Self {
        Vec::new().into()
    }
}

impl<I: AsIndex, T> Index<I> for IndexedVec<I, T> {
    type Output = T;

    fn index(&self, index: I) -> &Self::Output {
        &self.vec[index.to_usize()]
    }
}

impl<I: AsIndex, T> IndexMut<I> for IndexedVec<I, T> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.vec[index.to_usize()]
    }
}

impl<I, T> From<Vec<T>> for IndexedVec<I, T> {
    fn from(value: Vec<T>) -> Self {
        Self { vec: value, _marker: PhantomData }
    }
}

impl<I, T, const N: usize> From<[T; N]> for IndexedVec<I, T> {
    fn from(value: [T; N]) -> Self {
        Self { vec: value.into(), _marker: PhantomData }
    }
}

impl<I, T> FromIterator<T> for IndexedVec<I, T> {
    fn from_iter<IT: IntoIterator<Item = T>>(iter: IT) -> Self {
        Vec::from_iter(iter).into()
    }
}

impl<I, T> IntoIterator for IndexedVec<I, T> {
    type IntoIter = std::vec::IntoIter<T>;
    type Item = T;
    fn into_iter(self) -> Self::IntoIter {
        self.vec.into_iter()
    }
}

pub struct IndexedSet<I, T> {
    set: indexmap::IndexSet<T, rustc_hash::FxBuildHasher>,
    _marker: PhantomData<I>,
}

impl<I, T> Default for IndexedSet<I, T> {
    fn default() -> Self {
        Self { set: Default::default(), _marker: Default::default() }
    }
}

impl<I: AsIndex, T: Hash + Eq> IndexedSet<I, T> {
    pub fn insert_full(&mut self, value: T) -> (I, bool) {
        let (idx, is_new) = self.set.insert_full(value);
        (I::from_usize(idx), is_new)
    }

    pub fn get_index_of<Q>(&self, value: &Q) -> Option<I>
    where
        Q: indexmap::Equivalent<T> + Hash + ?Sized,
    {
        Some(I::from_usize(self.set.get_index_of(value)?))
    }

    pub fn index_of<Q>(&self, value: &Q) -> I
    where
        Q: indexmap::Equivalent<T> + Hash + ?Sized,
    {
        self.get_index_of(value).unwrap()
    }
}

impl<I, T> Deref for IndexedSet<I, T> {
    type Target = indexmap::IndexSet<T, rustc_hash::FxBuildHasher>;

    fn deref(&self) -> &Self::Target {
        &self.set
    }
}

impl<I, T> DerefMut for IndexedSet<I, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.set
    }
}

impl<I, T: Hash + Eq, const N: usize> From<[T; N]> for IndexedSet<I, T> {
    fn from(value: [T; N]) -> Self {
        Self { set: <_>::from_iter(value), _marker: PhantomData }
    }
}

impl<I: AsIndex, T> Index<I> for IndexedSet<I, T> {
    type Output = T;

    fn index(&self, index: I) -> &Self::Output {
        &self.set[index.to_usize()]
    }
}

pub trait AsIndex: Copy {
    fn to_usize(&self) -> usize;
    fn from_usize(index: usize) -> Self;
}

#[macro_export]
macro_rules! new_index {
    ($(#[$($meta:tt)*])* $vis:vis index $ty:ident) => {
        $(#[$($meta)*])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $vis struct $ty(pub usize);

        impl $crate::index::AsIndex for $ty {
            fn to_usize(&self) -> usize {
                self.0
            }

            fn from_usize(index: usize) -> Self {
                $ty(index)
            }
        }
    };
}
pub use new_index;

#[cfg(test)]
mod test {
    use crate::index::AsIndex;

    new_index!(pub index TestId);

    #[test]
    fn simple() {
        let id = TestId(42);
        assert_eq!(id.to_usize(), 42);

        let id = TestId::from_usize(42);
        assert_eq!(id.to_usize(), 42);
    }
}
