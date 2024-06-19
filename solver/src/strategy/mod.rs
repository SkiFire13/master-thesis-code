// TODO: Use node/vertex consistently

mod graph;
mod improve;
mod profile;
mod valuation;

#[cfg(test)]
mod test;

use std::iter;

use either::Either::*;
pub use graph::{GetRelevance, NodeId, ParityGraph, Player, Relevance, Reward};
pub use improve::improve;
pub use profile::PlayProfile;
pub use valuation::valuation;

pub type Set<T> = std::collections::BTreeSet<T>;
pub type NodeMap<T> = std::collections::HashMap<NodeId, T>;

pub trait Strategy {
    type Graph: ParityGraph;
    fn iter(&self, graph: &Self::Graph) -> impl Iterator<Item = (NodeId, NodeId)>;
    fn get_direct(&self, n: NodeId, graph: &Self::Graph) -> NodeId;
    fn get_inverse(&self, n: NodeId, graph: &Self::Graph) -> impl Iterator<Item = NodeId>;

    fn predecessors_of(&self, n: NodeId, graph: &Self::Graph) -> impl Iterator<Item = NodeId> {
        let p0_preds = self.get_inverse(n, graph);
        let p1_preds = graph.predecessors_of(n).filter(|&n| graph.player_of(n) == Player::P1);
        p0_preds.chain(p1_preds)
    }
    fn successors_of(&self, n: NodeId, graph: &Self::Graph) -> impl Iterator<Item = NodeId> {
        match graph.player_of(n) {
            Player::P0 => Left(iter::once(self.get_direct(n, graph))),
            Player::P1 => Right(graph.successors_of(n)),
        }
    }
}

pub trait StrategyMut: Strategy {
    fn update_each(&mut self, graph: &Self::Graph, f: impl FnMut(NodeId, NodeId) -> NodeId);
}
