use either::Either::{Left, Right};

use crate::strategy::game::{
    Game, GameStrategy, NodeId, NodeKind, NodeP0Id, NodeP1Id, Player, Relevance,
};

use super::improve::ImproveGraph;
use super::valuation::{Strategy, ValuationGraph};
use super::GetRelevance;

impl<'a> GetRelevance for Game {
    fn relevance_of(&self, u: NodeId) -> Relevance {
        (*self).relevance_of(u)
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

    fn player(&self, n: NodeId) -> crate::strategy::game::Player {
        match self.resolve(n) {
            NodeKind::L0 | NodeKind::W0 | NodeKind::P0(_) => Player::P0,
            NodeKind::L1 | NodeKind::W1 | NodeKind::P1(_) => Player::P1,
        }
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

impl Strategy for GameStrategy {
    type Graph = Game;

    fn iter(&self, game: &Self::Graph) -> impl Iterator<Item = (NodeId, NodeId)> {
        self.direct
            .iter()
            .enumerate()
            .map(|(n0, &n1)| (game.p0_ids[NodeP0Id(n0)], game.p1_ids[n1]))
            .chain([(NodeId::L0, NodeId::W1), (NodeId::W0, NodeId::L1)])
    }

    fn get(&self, n: NodeId, game: &Self::Graph) -> NodeId {
        match game.resolve(n) {
            NodeKind::L0 => NodeId::W1,
            NodeKind::W0 => NodeId::L1,
            NodeKind::P0(n) => game.p1_ids[self.direct[n]],
            NodeKind::L1 | NodeKind::W1 | NodeKind::P1(_) => unreachable!(),
        }
    }

    fn get_inverse(&self, n: NodeId, game: &Self::Graph) -> impl Iterator<Item = NodeId> {
        // TODO: The inverse of W1 could be a actual p0 node.
        match game.resolve(n) {
            NodeKind::L1 => Left([NodeId::W0].into_iter()),
            NodeKind::W1 => Left([NodeId::L0].into_iter()),
            NodeKind::P1(n) => Right(self.inverse[n].iter().map(|&n| game.p0_ids[n])),
            NodeKind::L0 | NodeKind::W0 | NodeKind::P0(_) => unreachable!(),
        }
    }
}
