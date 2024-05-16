use crate::strategy::game::{Game, NodeId, NodeKind, NodeP0Id, NodeP1Id, Relevance};

use super::improve::ImproveGraph;
use super::valuation::ValuationGraph;
use super::GetRelevance;

impl<'a> GetRelevance for Game {
    fn relevance_of(&self, u: NodeId) -> Relevance {
        (*self).relevance_of(u)
    }
}

impl<F: Fn(NodeId) -> Relevance> GetRelevance for F {
    fn relevance_of(&self, u: NodeId) -> Relevance {
        self(u)
    }
}

impl ImproveGraph for Game {
    fn p0_successors(&self, n: NodeP0Id) -> impl Iterator<Item = NodeP1Id> {
        self.p0_succs[n].iter().copied()
    }

    fn p1_to_node(&self, n: NodeP1Id) -> NodeId {
        self.p1_ids[n]
    }
}

impl ValuationGraph for Game {
    fn node_count(&self) -> usize {
        self.nodes.len()
    }

    fn p1_count(&self) -> usize {
        self.p1_set.len()
    }

    fn node_as_p0(&self, n: NodeId) -> Option<NodeP0Id> {
        match self.resolve(n) {
            NodeKind::P0(n) => Some(n),
            _ => None,
        }
    }
    fn node_as_p1(&self, n: NodeId) -> Option<NodeP1Id> {
        match self.resolve(n) {
            NodeKind::P1(n) => Some(n),
            _ => None,
        }
    }

    fn p0_to_node(&self, n: NodeP0Id) -> NodeId {
        self.p0_ids[n]
    }

    fn p1_to_node(&self, n: NodeP1Id) -> NodeId {
        self.p1_ids[n]
    }

    fn successors_of(&self, n: NodeId) -> impl Iterator<Item = NodeId> {
        self.successors_of(n)
    }

    fn predecessors_of(&self, n: NodeId) -> impl Iterator<Item = NodeId> {
        self.predecessors_of(n)
    }

    fn nodes_sorted_by_reward(&self) -> impl Iterator<Item = NodeId> {
        self.nodes_sorted_by_reward()
    }
}
