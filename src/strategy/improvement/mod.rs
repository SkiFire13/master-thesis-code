// TODO: Use node/vertex consistently

use super::game::NodeId;

// Bitset or something similar?
pub type Set<T> = std::collections::BTreeSet<T>;
pub type NodeMap<T> = std::collections::HashMap<NodeId, T>;

mod improve;
mod profile;
mod valuation;

mod impls;

pub use improve::improve;
pub use profile::{GetRelevance, PlayProfile};
pub use valuation::valuation;
