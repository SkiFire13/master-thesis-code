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
        self.p0.win[p0] = WinState::Win1;
        self.p0.w1.insert(p0);

        // Fixup strategy.
        strategy.update(p0, NodeP1Id::W1);
        final_strategy[self.p0.ids[p0]] = NodeId::W1;

        // Optimization: remove edges to successors.
        for p1 in std::mem::take(&mut self.p0.succs[p0]) {
            self.p1.preds[p1].swap_remove(&p0);
        }

        // Optimization: remove edges from predecessors and set them as winning.
        for p1 in std::mem::take(&mut self.p0.preds[p0]) {
            if self.p1.win[p1] != WinState::Win1 {
                self.set_p1_winning(p1, strategy, final_strategy);
            }
        }
    }

    pub fn set_p0_winning(
        &mut self,
        p0: NodeP0Id,
        strategy: &mut GameStrategy,
        final_strategy: &mut IndexedVec<NodeId, NodeId>,
    ) {
        self.p0.win[p0] = WinState::Win0;
        self.p0.w0.insert(p0);
        self.p0.incomplete.swap_remove(&p0);

        // Fixup strategy.
        strategy.update(p0, NodeP1Id::L1);
        final_strategy[self.p0.ids[p0]] = NodeId::L1;

        // Optimization: remove edges to successors.
        for p1 in std::mem::take(&mut self.p0.succs[p0]) {
            self.p1.preds[p1].swap_remove(&p0);
        }

        // Optimization: remove edges from predecessors.
        for p1 in std::mem::take(&mut self.p0.preds[p0]) {
            // Remove them only if the edge is not currently in the final strategy.
            if final_strategy[self.p1.ids[p1]] == self.p0.ids[p0] {
                // If it is, and it is also the only remaining edge, then it is losing.
                if self.p1.succs[p1].len() == 1 && self.p1.moves[p1].is_exhausted() {
                    self.set_p1_losing(p1, strategy, final_strategy);
                } else {
                    self.p0.preds[p0].insert(p1);
                }
            } else {
                self.p1.succs[p1].swap_remove(&p0);
            }
        }
    }

    pub fn set_p1_losing(
        &mut self,
        p1: NodeP1Id,
        strategy: &mut GameStrategy,
        final_strategy: &mut IndexedVec<NodeId, NodeId>,
    ) {
        self.p1.win[p1] = WinState::Win0;
        self.p1.w0.insert(p1);

        // Fixup strategy.
        final_strategy[self.p1.ids[p1]] = NodeId::W0;

        // Optimization: remove edges to successors.
        for p0 in std::mem::take(&mut self.p1.succs[p1]) {
            self.p0.preds[p0].swap_remove(&p1);
        }

        // Optimization: remove edges from predecessors and set them as winning.
        for p0 in std::mem::take(&mut self.p1.preds[p1]) {
            if self.p0.win[p0] != WinState::Win0 {
                self.set_p0_winning(p0, strategy, final_strategy);
            }
        }
    }

    pub fn set_p1_winning(
        &mut self,
        p1: NodeP1Id,
        strategy: &mut GameStrategy,
        final_strategy: &mut IndexedVec<NodeId, NodeId>,
    ) {
        self.p1.win[p1] = WinState::Win1;
        self.p1.w1.insert(p1);
        self.p1.incomplete.swap_remove(&p1);

        // Fixup strategy.
        final_strategy[self.p1.ids[p1]] = NodeId::L0;

        // Optimization: remove edges to successors.
        for p0 in std::mem::take(&mut self.p1.succs[p1]) {
            self.p0.preds[p0].swap_remove(&p1);
        }

        // Optimization: remove edges from predecessors.
        for p0 in std::mem::take(&mut self.p1.preds[p1]) {
            // Remove them only if the edge is not currently in the final strategy.
            if final_strategy[self.p1.ids[p1]] == self.p0.ids[p0] {
                // If it is, and it is also the only remaining edge, then it is losing.
                if self.p1.succs[p1].len() == 1 && self.p1.moves[p1].is_exhausted() {
                    self.set_p1_losing(p1, strategy, final_strategy);
                } else {
                    self.p1.preds[p1].insert(p0);
                }
            } else {
                self.p0.succs[p0].swap_remove(&p1);
            }
        }
    }
}
