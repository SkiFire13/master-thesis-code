// TODO: Use node/vertex consistently

mod profile;
mod strategy;
mod valuation;

#[cfg(test)]
mod test;

use std::cmp::Reverse;
use std::hash::Hash;

use profile::PlayProfile;

use crate::index::{AsIndex, IndexedVec};
use crate::solver::{NodeId, Player, Solver};
use crate::{new_index, Set};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Priority(usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Relevance(Priority, SNodeId);

impl Relevance {
    fn winner(self) -> Player {
        match self.0 .0 % 2 {
            0 => Player::P0,
            _ => Player::P1,
        }
    }

    fn reward(self) -> Reward {
        match self.winner() {
            Player::P0 => Reward::P0(self),
            Player::P1 => Reward::P1(Reverse(self)),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reward {
    P1(Reverse<Relevance>),
    Neutral,
    P0(Relevance),
}

struct StrategySolver {
    node_to_snode: IndexedVec<NodeId, SNodeId>,
    snode_to_node: IndexedVec<SNodeId, NodeId>,

    players: IndexedVec<SNodeId, Player>,
    priorities: IndexedVec<SNodeId, Priority>,

    preds: IndexedVec<SNodeId, Set<SNodeId>>,
    succs: IndexedVec<SNodeId, Set<SNodeId>>,

    strategy: IndexedVec<SNodeId, SNodeId>,
    inverse_strategy: IndexedVec<SNodeId, Set<SNodeId>>,

    profiles: IndexedVec<SNodeId, PlayProfile>,
    evaluated: IndexedVec<SNodeId, bool>,
}

new_index!(pub index SNodeId);

impl SNodeId {
    const W0: SNodeId = SNodeId(0);
    const L0: SNodeId = SNodeId(1);
    const W1: SNodeId = SNodeId(2);
    const L1: SNodeId = SNodeId(3);

    const INVALID: SNodeId = SNodeId(usize::MAX);
}

impl StrategySolver {
    fn default_succ(player: Player) -> SNodeId {
        match player {
            Player::P0 => SNodeId::W1,
            Player::P1 => SNodeId::W0,
        }
    }

    fn new() -> Self {
        Self {
            node_to_snode: IndexedVec::new(),
            snode_to_node: IndexedVec::from([NodeId::INVALID; 4]),

            players: IndexedVec::from([Player::P0, Player::P0, Player::P1, Player::P1]),
            priorities: IndexedVec::from([Priority(0), Priority(1), Priority(0), Priority(1)]),

            preds: IndexedVec::from([
                Set::from_iter([SNodeId::L1]),
                Set::from_iter([SNodeId::W1]),
                Set::from_iter([SNodeId::L0]),
                Set::from_iter([SNodeId::W0]),
            ]),
            succs: IndexedVec::from([
                Set::from_iter([SNodeId::L1]),
                Set::from_iter([SNodeId::W1]),
                Set::from_iter([SNodeId::L0]),
                Set::from_iter([SNodeId::W0]),
            ]),

            strategy: IndexedVec::from([SNodeId::L1, SNodeId::W1, SNodeId::L0, SNodeId::W0]),
            inverse_strategy: IndexedVec::from([
                Set::from_iter([SNodeId::L1]),
                Set::from_iter([SNodeId::W1]),
                Set::from_iter([SNodeId::L0]),
                Set::from_iter([SNodeId::W0]),
            ]),

            profiles: IndexedVec::from(vec![PlayProfile::default(); 4]),
            evaluated: IndexedVec::default(),
        }
    }

    fn add_node(&mut self, player: Player, priority: usize) -> NodeId {
        let node = self.node_to_snode.push(SNodeId::INVALID);
        let snode = self.snode_to_node.push(node);
        self.node_to_snode[node] = snode;

        self.players.push(player);
        self.priorities.push(Priority(priority));

        self.preds.push(Set::default());
        self.succs.push(Set::default());

        let default_succ = Self::default_succ(player);
        self.add_edge(snode, default_succ);

        self.strategy.push(default_succ);
        self.inverse_strategy[default_succ].insert(snode);
        self.inverse_strategy.push(Set::default());

        self.profiles.push(PlayProfile::losing_for_player(player));

        node
    }

    fn add_edge(&mut self, u: SNodeId, v: SNodeId) {
        debug_assert_ne!(self.players[u], self.players[v]);

        let default_succ = Self::default_succ(self.players[u]);
        let first_edge = self.succs[u].len() == 1 && self.succs[u][0] == default_succ;

        self.preds[v].insert(u);
        self.succs[u].insert(v);

        if first_edge {
            self.remove_edge(u, default_succ);
        }
    }

    fn remove_edge(&mut self, u: SNodeId, v: SNodeId) {
        self.preds[v].swap_remove(&u);
        self.succs[u].swap_remove(&v);

        if self.succs[u].is_empty() {
            let default_succ = Self::default_succ(self.players[u]);
            self.succs[u].insert(default_succ);
            self.preds[default_succ].insert(u);
        }

        if self.strategy[u] == v {
            let w = self.succs[u][0];
            self.strategy[u] = w;
            self.inverse_strategy[v].swap_remove(&u);
            self.inverse_strategy[w].insert(u);
        }
    }

    fn remove_node(&mut self, u: SNodeId) {
        for i in (0..self.preds[u].len()).rev() {
            // May need to fixup the successors.
            self.remove_edge(self.preds[u][i], u);
        }

        for v in std::mem::take(&mut self.succs[u]) {
            self.preds[v].swap_remove(&u);
        }

        let v = self.strategy[u];
        self.inverse_strategy[v].swap_remove(&u);

        let node = self.snode_to_node[u];
        self.node_to_snode[node] = SNodeId::INVALID;

        let i = u.to_usize();
        self.snode_to_node.swap_remove(i);
        self.players.swap_remove(i);
        self.priorities.swap_remove(i);
        self.preds.swap_remove(i);
        self.succs.swap_remove(i);
        self.strategy.swap_remove(i);
        self.inverse_strategy.swap_remove(i);
        self.profiles.swap_remove(i);

        let last = SNodeId(self.players.len());
        if u != last {
            for &v in &self.preds[u] {
                self.succs[v].insert(u);
                self.succs[v].swap_remove(&last);
            }

            for &v in &self.succs[u] {
                self.preds[v].insert(u);
                self.preds[v].swap_remove(&last);
            }

            let v = self.strategy[u];
            self.inverse_strategy[v].insert(u);
            self.inverse_strategy[v].swap_remove(&last);
        }
    }

    fn solve(&mut self) {
        let mut improved = true;
        while improved {
            self.valuation();
            improved = self.improve();
        }
    }

    fn relevance_of(&self, n: SNodeId) -> Relevance {
        Relevance(self.priorities[n], n)
    }

    fn reward_of(&self, n: SNodeId) -> Reward {
        self.relevance_of(n).reward()
    }

    fn winner(&self, n: SNodeId) -> Player {
        self.profiles[n].winner(self)
    }
}

impl Solver for StrategySolver {
    fn new() -> Self {
        StrategySolver::new()
    }

    fn add_node(&mut self, player: Player, priority: usize) -> NodeId {
        self.add_node(player, priority)
    }

    fn add_edge(&mut self, u: NodeId, v: NodeId) {
        self.add_edge(self.node_to_snode[u], self.node_to_snode[v]);
    }

    fn remove_edge(&mut self, u: NodeId, v: NodeId) {
        self.remove_edge(self.node_to_snode[u], self.node_to_snode[v]);
    }

    fn remove_node(&mut self, u: NodeId) {
        self.remove_node(self.node_to_snode[u]);
    }

    fn predecessors(&self, n: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.preds[self.node_to_snode[n]]
            .iter()
            .filter(|&&n| n > SNodeId::L1)
            .map(|&n| self.snode_to_node[n])
    }

    fn player(&self, n: NodeId) -> Player {
        self.players[self.node_to_snode[n]]
    }

    fn solve(&mut self) {
        self.solve()
    }

    fn winner(&self, n: NodeId) -> Player {
        self.winner(self.node_to_snode[n])
    }

    fn inverse_strategy(&self, n: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.inverse_strategy[self.node_to_snode[n]].iter().map(|&n| self.snode_to_node[n])
    }
}
