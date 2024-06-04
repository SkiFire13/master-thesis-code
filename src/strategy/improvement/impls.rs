use either::Either::{Left, Right};

use crate::strategy::game::{Game, GameStrategy, NodeId, NodeKind, Player, Relevance};

use super::improve::StrategyMut;
use super::valuation::{Strategy, ValuationGraph};
use super::GetRelevance;

impl<'a> GetRelevance for Game {
    fn relevance_of(&self, u: NodeId) -> Relevance {
        (*self).relevance_of(u)
    }
}

impl ValuationGraph for Game {
    fn node_count(&self) -> usize {
        self.nodes.len()
    }

    fn player(&self, n: NodeId) -> Player {
        self.controlling_player(n)
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
            .map(|(n0, &n1)| {
                (game.p0.node_ids[n0], n1.map_or(NodeId::W1, |n1| game.p1.node_ids[n1]))
            })
            .chain([(NodeId::L0, NodeId::W1), (NodeId::W0, NodeId::L1)])
    }

    fn get_direct(&self, n: NodeId, game: &Self::Graph) -> NodeId {
        match game.resolve(n) {
            NodeKind::L0 => NodeId::W1,
            NodeKind::W0 => NodeId::L1,
            NodeKind::P0(n) => self.direct[n].map_or(NodeId::W1, |n| game.p1.node_ids[n]),
            NodeKind::L1 | NodeKind::W1 | NodeKind::P1(_) => unreachable!(),
        }
    }

    fn get_inverse(&self, n: NodeId, game: &Self::Graph) -> impl Iterator<Item = NodeId> {
        let kind = game.resolve(n);

        // Predecessors given by the strategy
        let p0 = match kind {
            NodeKind::W1 => Left(&self.inverse_w1),
            NodeKind::P1(n) => Left(&self.inverse[n]),
            _ => Right([].into_iter()),
        }
        .map_left(|s| s.iter().map(|&n| game.p0.node_ids[n]));

        // Predecessors of the special nodes (not included in the "stored" strategy)
        let wl = match kind {
            NodeKind::W1 => [NodeId::L0].iter().copied(),
            NodeKind::L1 => [NodeId::W0].iter().copied(),
            _ => [].iter().copied(),
        };

        p0.chain(wl)
    }
}

impl StrategyMut for GameStrategy {
    fn update_each(&mut self, graph: &Self::Graph, mut f: impl FnMut(NodeId, NodeId) -> NodeId) {
        for (p0, p1) in self.direct.enumerate_mut() {
            let n0 = graph.p0.node_ids[p0];
            let n1 = match *p1 {
                Some(p1) => graph.p1.node_ids[p1],
                None => NodeId::W1,
            };

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
                NodeKind::W1 => {
                    // p1 cannot be None otherwise n2 == n1 would be true
                    self.inverse[p1.unwrap()].remove(&p0);
                    self.inverse_w1.insert(p0);
                    *p1 = None;
                }
                NodeKind::P1(np1) => {
                    // Update the inverse sets.
                    match *p1 {
                        Some(p1) => self.inverse[p1].remove(&p0),
                        None => self.inverse_w1.remove(&p0),
                    };
                    self.inverse[np1].insert(p0);
                    // Update the direct successor.
                    *p1 = Some(np1);
                }
            }
        }
    }
}
