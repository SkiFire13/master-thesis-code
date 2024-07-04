use crate::solver::{NodeId, Player, Solver};
use crate::Set;

use super::{Expander, LocalSolver};

impl<S: Solver, E: Expander> LocalSolver<S, E> {
    pub(super) fn update_winning_sets(&mut self) {
        let mut escaping = Set::default();
        escaping.extend(self.boundary_p0.iter().copied());
        escaping.extend(self.boundary_p1.iter().copied());

        let mut queue = escaping.iter().map(|&id| self.nodes[id]).collect::<Vec<_>>();

        while let Some(n) = queue.pop() {
            for m in self.solver.inverse_strategy(n) {
                for p in self.solver.predecessors(m) {
                    if escaping.insert(self.node_to_id[&p]) {
                        queue.push(p);
                    }
                }
            }
        }

        for id in self.nodes.indexes() {
            let node = self.nodes[id];
            if node == NodeId::INVALID {
                continue;
            }

            let winner = self.solver.winner(node);
            let player = self.solver.player(node);

            if winner == player {
                continue;
            }

            queue.push(node);
            self.nodes[id] = NodeId::INVALID;
            self.win[id] = Some(winner);
            match player {
                Player::P0 => _ = self.boundary_p0.swap_remove(&id),
                Player::P1 => _ = self.boundary_p1.swap_remove(&id),
            }

            for pred in self.solver.predecessors(node) {
                let id_pred = self.node_to_id[&pred];

                queue.push(pred);
                self.nodes[id_pred] = NodeId::INVALID;
                self.win[id_pred] = Some(winner);
                match winner {
                    Player::P0 => _ = self.boundary_p0.swap_remove(&id_pred),
                    Player::P1 => _ = self.boundary_p1.swap_remove(&id_pred),
                }
            }
        }

        for node in queue {
            self.solver.remove_node(node);
        }
    }
}
