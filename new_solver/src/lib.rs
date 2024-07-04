pub mod index;
pub mod local;
mod retain;
pub mod solver;
pub mod strategy;
pub mod symbolic;

pub type Set<T> = indexmap::IndexSet<T, rustc_hash::FxBuildHasher>;
