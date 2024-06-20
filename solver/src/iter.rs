pub enum Simplify<T> {
    Keep(T),
    Remove,
    Break,
}

pub trait IteratorExt: Iterator + Sized {
    fn simplify_with<F, G, U>(self, mut g: G, f: F) -> impl Iterator<Item = Option<U>>
    where
        G: FnMut(usize),
        F: FnMut(Self::Item) -> Simplify<U>,
    {
        self.map(f)
            .enumerate()
            .inspect(move |(i, s)| match s {
                Simplify::Remove => g(*i),
                _ => {}
            })
            .filter_map(|(_, s)| match s {
                Simplify::Keep(item) => Some(Some(item)),
                Simplify::Remove => None,
                Simplify::Break => Some(None),
            })
    }

    fn simplify<F, U>(self, f: F) -> impl Iterator<Item = Option<U>>
    where
        F: FnMut(Self::Item) -> Simplify<U>,
    {
        self.simplify_with(|_| {}, f)
    }
}

impl<I: Iterator> IteratorExt for I {}
