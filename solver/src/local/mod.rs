mod escape;
mod expansion;
mod game;
mod impls;
mod solve;
mod winning;

// Bitset or something similar?
pub type Set<T> = std::collections::BTreeSet<T>;

pub use solve::solve;
