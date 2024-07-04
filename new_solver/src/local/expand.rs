use crate::local::Inserted;
use crate::solver::{Player, Solver};

use super::{Expander, LNodeId, LocalSolver};

impl<S: Solver, E: Expander> LocalSolver<S, E> {
    pub(super) fn expand(&mut self, init: LNodeId) {
        let goal = self.explore_goal;
        self.explore_goal *= 2;

        let mut explored = 0;
        while explored < goal {
            let start = match self.solver.winner(self.nodes[init]) {
                Player::P0 => self.boundary_p1.last(),
                Player::P1 => self.boundary_p0.last(),
            };

            let Some(&start) = start else { return };

            // TODO: Simplify moves?

            let mut curr = start;

            loop {
                let Some(next) = self.moves[curr].next() else {
                    match self.solver.player(self.nodes[curr]) {
                        Player::P0 => _ = self.boundary_p0.swap_remove(&curr),
                        Player::P1 => _ = self.boundary_p1.swap_remove(&curr),
                    }

                    break;
                };

                let inserted = self.insert(next);

                self.solver.add_edge(self.nodes[curr], self.nodes[inserted.id()]);

                match inserted {
                    Inserted::New(next) => curr = next,
                    Inserted::Existing(_) => break,
                }

                explored += 1;
            }
        }
    }
}
