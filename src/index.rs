use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};

pub struct IndexVec<I, T> {
    vec: Vec<T>,
    _marker: PhantomData<I>,
}

impl<I, T> Deref for IndexVec<I, T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.vec
    }
}

impl<I, T> DerefMut for IndexVec<I, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vec
    }
}

impl<I: AsIndex, T> IndexVec<I, T> {
    pub fn new() -> Self {
        Vec::new().into()
    }

    pub fn push(&mut self, value: T) -> I {
        let index = I::from_usize(self.len());
        self.vec.push(value);
        index
    }
}

impl<I: AsIndex, T> Index<I> for IndexVec<I, T> {
    type Output = T;

    fn index(&self, index: I) -> &Self::Output {
        &self.vec[index.to_usize()]
    }
}

impl<I: AsIndex, T> IndexMut<I> for IndexVec<I, T> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.vec[index.to_usize()]
    }
}

impl<I, T> From<Vec<T>> for IndexVec<I, T> {
    fn from(value: Vec<T>) -> Self {
        Self { vec: value, _marker: PhantomData }
    }
}

impl<I, T> FromIterator<T> for IndexVec<I, T> {
    fn from_iter<IT: IntoIterator<Item = T>>(iter: IT) -> Self {
        Vec::from_iter(iter).into()
    }
}

pub struct IndexSet<I, T> {
    set: indexmap::IndexSet<T>,
    _marker: PhantomData<I>,
}

impl<I, T> IndexSet<I, T> {
    pub fn new() -> Self {
        Self {
            set: indexmap::IndexSet::new(),
            _marker: PhantomData,
        }
    }
}

impl<I, T> Deref for IndexSet<I, T> {
    type Target = indexmap::IndexSet<T>;

    fn deref(&self) -> &Self::Target {
        &self.set
    }
}

impl<I, T> DerefMut for IndexSet<I, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.set
    }
}

impl<I, T: Hash + Eq, const N: usize> From<[T; N]> for IndexSet<I, T> {
    fn from(value: [T; N]) -> Self {
        Self { set: value.into(), _marker: PhantomData }
    }
}

impl<I: AsIndex, T> Index<I> for IndexSet<I, T> {
    type Output = T;

    fn index(&self, index: I) -> &Self::Output {
        &self.set[index.to_usize()]
    }
}

pub trait AsIndex {
    fn to_usize(&self) -> usize;
    fn from_usize(index: usize) -> Self;
}

macro_rules! new_index {
    ($(#[$($meta:tt)*])* $vis:vis index $ty:ident) => {
        $(#[$($meta)*])*
        #[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        $vis struct $ty(pub usize);

        impl crate::index::AsIndex for $ty {
            fn to_usize(&self) -> usize {
                self.0
            }

            fn from_usize(index: usize) -> Self {
                Self(index)
            }
        }
    };
}
pub(crate) use new_index;
