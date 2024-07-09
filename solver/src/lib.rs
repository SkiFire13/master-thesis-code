pub mod index;
pub mod local;
mod retain;
pub mod strategy;
pub mod symbolic;

pub type Set<T> = indexmap::IndexSet<T, rustc_hash::FxBuildHasher>;
pub type Map<K, V> = rustc_hash::FxHashMap<K, V>;
