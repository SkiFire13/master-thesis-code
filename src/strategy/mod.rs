pub mod escape;
pub mod expansion;
pub mod game;
pub mod improvement;
pub mod solve;

// Bitset or something similar?
pub type Set<T> = std::collections::BTreeSet<T>;
pub type NodeMap<T> = std::collections::HashMap<game::NodeId, T>;
