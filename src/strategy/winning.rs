use crate::strategy::game::WinState;

use super::game::{Game, GameStrategy, NodeP0Id, NodeP1Id};

impl Game {
    pub fn set_p0_losing(&mut self, p0: NodeP0Id, strategy: &mut GameStrategy) {
        // Mark nodes as losing.
        self.p0.win[p0] = WinState::Win1;
        self.p0.w1.insert(p0);

        // Fixup P0 strategy
        strategy.update(p0, NodeP1Id::W1);

        // Optimization: remove successors of predecessors
        for p1 in std::mem::take(&mut self.p0.preds[p0]) {
            debug_assert_eq!(self.p1.win[p1], WinState::Unknown);

            // Mark predecessors as winning.
            self.p1.win[p1] = WinState::Win1;
            self.p1.w1.insert(p1);

            // Optimization: remove successors of predecessors
            for p0 in std::mem::take(&mut self.p1.succs[p1]) {
                self.p0.preds[p0].remove(&p1);
            }
        }

        // Optimization: remove successors
        for p1 in std::mem::take(&mut self.p0.succs[p0]) {
            self.p1.preds[p1].remove(&p0);
        }
    }

    pub fn set_p1_losing(&mut self, p1: NodeP1Id, strategy: &mut GameStrategy) {
        // Mark nodes as losing.
        self.p1.win[p1] = WinState::Win0;
        self.p1.w0.insert(p1);

        // Optimization: remove successors of predecessors
        for p0 in std::mem::take(&mut self.p1.preds[p1]) {
            debug_assert_eq!(self.p0.win[p0], WinState::Unknown);

            // Mark predecessors as winning.
            self.p0.win[p0] = WinState::Win0;
            self.p0.w0.insert(p0);

            // Optimization: remove successors of predecessors
            for p1 in std::mem::take(&mut self.p0.succs[p0]) {
                self.p1.preds[p1].remove(&p0);
            }

            // Fixup P0 strategy
            strategy.update(p0, NodeP1Id::L1);
        }

        // Optimization: remove successors
        for p0 in std::mem::take(&mut self.p1.succs[p1]) {
            self.p0.preds[p0].remove(&p1);
        }
    }
}
