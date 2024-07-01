use crate::index::IndexedVec;
use crate::strategy::NodeId;

use super::game::{Game, GameStrategy, NodeP0Id, NodeP1Id, WinState};

impl Game {
    pub fn set_p0_losing(
        &mut self,
        p0: NodeP0Id,
        strategy: &mut GameStrategy,
        final_strategy: &mut IndexedVec<NodeId, NodeId>,
    ) {
        // Mark nodes as losing.
        self.p0.win[p0] = WinState::Win1;
        self.p0.w1.insert(p0);

        // Fixup strategy
        strategy.update(p0, NodeP1Id::W1);
        final_strategy[self.p0.ids[p0]] = NodeId::W1;

        // Optimization: remove successors
        for p1 in std::mem::take(&mut self.p0.succs[p0]) {
            self.p1.preds[p1].remove(&p0);
        }

        // Optimization: remove successors of predecessors
        for p1 in std::mem::take(&mut self.p0.preds[p0]) {
            debug_assert_eq!(self.p1.win[p1], WinState::Unknown);
            self.set_p1_winning(p1, strategy, final_strategy);
        }
    }

    pub fn set_p0_winning(
        &mut self,
        p0: NodeP0Id,
        strategy: &mut GameStrategy,
        final_strategy: &mut IndexedVec<NodeId, NodeId>,
    ) {
        // Mark predecessors as winning and complete.
        self.p0.win[p0] = WinState::Win0;
        self.p0.w0.insert(p0);
        self.p0.incomplete.swap_remove(&p0);
        // Fixup strategy
        strategy.update(p0, NodeP1Id::L1);
        final_strategy[self.p0.ids[p0]] = NodeId::L1;

        // Optimization: remove successors of predecessors
        for p1 in std::mem::take(&mut self.p0.succs[p0]) {
            self.p1.preds[p1].remove(&p0);
        }

        // Optimization: remove predecessors of predecessors
        for p1 in std::mem::take(&mut self.p0.preds[p0]) {
            self.p1.succs[p1].remove(&p0);
            if self.p1.succs[p1].is_empty() {
                final_strategy[self.p1.ids[p1]] = NodeId::W0;
            }
        }
    }

    pub fn set_p1_losing(
        &mut self,
        p1: NodeP1Id,
        strategy: &mut GameStrategy,
        final_strategy: &mut IndexedVec<NodeId, NodeId>,
    ) {
        // Mark nodes as losing.
        self.p1.win[p1] = WinState::Win0;
        self.p1.w0.insert(p1);

        // Fixup strategy
        final_strategy[self.p1.ids[p1]] = NodeId::W0;

        // Optimization: remove successors
        for p0 in std::mem::take(&mut self.p1.succs[p1]) {
            self.p0.preds[p0].remove(&p1);
        }

        // Optimization: remove successors of predecessors
        for p0 in std::mem::take(&mut self.p1.preds[p1]) {
            debug_assert_eq!(self.p0.win[p0], WinState::Unknown);
            self.set_p0_winning(p0, strategy, final_strategy);
        }
    }

    pub fn set_p1_winning(
        &mut self,
        p1: NodeP1Id,
        strategy: &mut GameStrategy,
        final_strategy: &mut IndexedVec<NodeId, NodeId>,
    ) {
        // Mark predecessors as winning and complete.
        self.p1.win[p1] = WinState::Win1;
        self.p1.w1.insert(p1);
        self.p1.incomplete.swap_remove(&p1);
        // Fixup strategy
        final_strategy[self.p1.ids[p1]] = NodeId::L0;

        // Optimization: remove successors of predecessors
        for p0 in std::mem::take(&mut self.p1.succs[p1]) {
            self.p0.preds[p0].remove(&p1);
        }

        // Optimization: remove predecessors of predecessors
        for p0 in std::mem::take(&mut self.p1.preds[p1]) {
            self.p0.succs[p0].remove(&p1);

            if strategy.direct[p0] == p1 {
                if let Some(&p1) = self.p0.succs[p0].iter().next() {
                    strategy.update(p0, p1);
                    final_strategy[self.p0.ids[p0]] = self.p1.ids[p1];
                } else {
                    strategy.update(p0, NodeP1Id::W1);
                    final_strategy[self.p0.ids[p0]] = NodeId::W1;
                }
            }
        }
    }
}
