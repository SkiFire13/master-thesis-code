use either::Either::{Left, Right};

use crate::strategy::game::{
    Game, GameStrategy, NodeId, NodeKind, NodeP0Id, NodeP1Id, Player, Relevance,
};

use super::improve::{ImproveGraph, StrategyMut};
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
            .enumerate()
            .map(|(n0, &n1)| (game.p0_ids[n0], game.p1_ids[n1]))
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
            NodeKind::L1 => Left([NodeId::W0].iter().copied()),
            NodeKind::W1 => Left([NodeId::L0].iter().copied()),
            NodeKind::P1(n) => Right(self.inverse[n].iter().map(|&n| game.p0_ids[n])),
            NodeKind::L0 | NodeKind::W0 | NodeKind::P0(_) => Left([].iter().copied()),
        }
    }
}

impl StrategyMut for GameStrategy {
    fn update_each(&mut self, graph: &Self::Graph, mut f: impl FnMut(NodeId, NodeId) -> NodeId) {
        for (p0, p1) in self.direct.enumerate_mut() {
            let n0 = graph.p0_ids[p0];
            let n1 = graph.p1_ids[*p1];

            let n2 = f(n0, n1);

            // Successor didn't change, nothing to update.
            if n2 == n1 {
                continue;
            }

            match graph.resolve(n2) {
                // P0 nodes cannot reach other P0 nodes.
                NodeKind::L0 | NodeKind::W0 | NodeKind::P0(_) => unreachable!(),
                // Only W0 can reach L1 but we skipped it.
                NodeKind::L1 => unreachable!(),
                NodeKind::W1 => todo!(),
                NodeKind::P1(np1) => {
                    // Update the inverse sets.
                    self.inverse[*p1].remove(&p0);
                    self.inverse[np1].insert(p0);
                    // Update the direct successor.
                    *p1 = np1;
                }
            }
        }
    }
}
