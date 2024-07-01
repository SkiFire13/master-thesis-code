use either::Either::{Left, Right};

use crate::index::IndexedVec;
use crate::strategy::{
    GetRelevance, NodeId, ParityGraph, Player, Relevance, Strategy, StrategyMut,
};

use super::game::{Game, GameStrategy, NodeKind, NodeP1Id};

impl<'a> GetRelevance for Game {
    fn relevance_of(&self, u: NodeId) -> Relevance {
        (*self).relevance_of(u)
    }
}

impl ParityGraph for Game {
    fn node_count(&self) -> usize {
        self.nodes.len()
    }

    fn player_of(&self, n: NodeId) -> Player {
        self.player_of(n)
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

fn p1_to_node(p1: NodeP1Id, ids: &IndexedVec<NodeP1Id, NodeId>) -> NodeId {
    match p1 {
        NodeP1Id::W1 => NodeId::W1,
        NodeP1Id::L1 => NodeId::L1,
        p1 => ids[p1],
    }
}

impl Strategy for GameStrategy {
    type Graph = Game;

    fn iter(&self, game: &Self::Graph) -> impl Iterator<Item = (NodeId, NodeId)> {
        self.direct
            .enumerate()
            .map(|(p0, &p1)| (game.p0.ids[p0], p1_to_node(p1, &game.p1.ids)))
            .chain([(NodeId::L0, NodeId::W1), (NodeId::W0, NodeId::L1)])
    }

    fn get_direct(&self, n: NodeId, game: &Self::Graph) -> NodeId {
        match game.resolve(n) {
            NodeKind::L0 => NodeId::W1,
            NodeKind::W0 => NodeId::L1,
            NodeKind::P0(n) => p1_to_node(self.direct[n], &game.p1.ids),
            NodeKind::L1 | NodeKind::W1 | NodeKind::P1(_) => unreachable!(),
        }
    }

    fn get_inverse(&self, n: NodeId, game: &Self::Graph) -> impl Iterator<Item = NodeId> {
        let map_p0 = |&n| game.p0.ids[n];
        match game.resolve(n) {
            NodeKind::L1 => Left(self.inverse_l1.iter().map(map_p0).chain([NodeId::W0])),
            NodeKind::W1 => Left(self.inverse_w1.iter().map(map_p0).chain([NodeId::L0])),
            NodeKind::P1(n) => Right(self.inverse[n].iter().map(map_p0)),
            NodeKind::L0 | NodeKind::W0 | NodeKind::P0(_) => unreachable!(),
        }
    }

    fn predecessors_of(&self, n: NodeId, game: &Self::Graph) -> impl Iterator<Item = NodeId> {
        match game.player_of(n) {
            Player::P0 => Left(game.predecessors_of(n)),
            Player::P1 => Right(self.get_inverse(n, game)),
        }
    }

    fn successors_of(&self, n: NodeId, game: &Self::Graph) -> impl Iterator<Item = NodeId> {
        match game.player_of(n) {
            Player::P0 => Left([self.get_direct(n, game)].into_iter()),
            Player::P1 => Right(game.successors_of(n)),
        }
    }
}

impl StrategyMut for GameStrategy {
    fn update_each(&mut self, graph: &Self::Graph, mut f: impl FnMut(NodeId, NodeId) -> NodeId) {
        for (p0, p1) in self.direct.enumerate_mut() {
            // Convert p1 to an id, handling virtual ids
            let n1 = match *p1 {
                NodeP1Id::W1 => NodeId::W1,
                NodeP1Id::L1 => NodeId::L1,
                p1 => graph.p1.ids[p1],
            };

            // Perform the callback
            let nn1 = f(graph.p0.ids[p0], n1);

            // Successor didn't change, nothing to update.
            if nn1 == n1 {
                continue;
            }

            // Remove the old edge from the inverse strategy
            match *p1 {
                NodeP1Id::W1 => _ = self.inverse_w1.swap_remove(&p0),
                NodeP1Id::L1 => _ = self.inverse_l1.swap_remove(&p0),
                p1 => _ = self.inverse[p1].swap_remove(&p0),
            }

            // Update p1 by reverse resolving nn1
            *p1 = match graph.resolve(nn1) {
                // P0 nodes cannot reach other P0 nodes.
                NodeKind::L0 | NodeKind::W0 | NodeKind::P0(_) => unreachable!(),
                NodeKind::L1 => NodeP1Id::L1,
                NodeKind::W1 => NodeP1Id::W1,
                NodeKind::P1(np1) => np1,
            };

            // Insert the new edge in the inverse strategy
            match *p1 {
                NodeP1Id::W1 => _ = self.inverse_w1.insert(p0),
                NodeP1Id::L1 => _ = self.inverse_l1.insert(p0),
                p1 => _ = self.inverse[p1].insert(p0),
            }
        }
    }
}
