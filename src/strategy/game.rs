use std::cmp::Reverse;

use either::Either::{Left, Right};

use crate::index::{new_index, AsIndex, IndexedSet, IndexedVec};
use crate::symbolic::compose::EqsFormulas;
use crate::symbolic::eq::{FixType, VarId};
use crate::symbolic::formula::{BasisElemId, Formula};
use crate::symbolic::moves::{P0Moves, P0Pos, P1Moves, P1Pos};

use super::Set;

new_index!(pub index NodeId);

impl NodeId {
    pub const W0: NodeId = NodeId(0);
    pub const L0: NodeId = NodeId(1);
    pub const W1: NodeId = NodeId(2);
    pub const L1: NodeId = NodeId(3);

    pub const INIT: NodeId = NodeId(4);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NodeKind {
    L0,
    L1,
    W0,
    W1,
    P0(NodeP0Id),
    P1(NodeP1Id),
}

new_index!(pub index NodeP0Id);
new_index!(pub index NodeP1Id);

impl NodeP0Id {
    pub const INIT: NodeP0Id = NodeP0Id(0);
}

impl NodeP1Id {
    pub const W1: NodeP1Id = NodeP1Id(usize::MAX);
    pub const L1: NodeP1Id = NodeP1Id(usize::MAX - 1);
}

#[derive(Debug, PartialEq, Eq)]
pub enum WinState {
    Unknown,
    Win0,
    Win1,
}

// Group of informations for a player nodes
pub struct NodesData<I, P, M, O> {
    // Deduplicates positions and maps them to a numeric id.
    pub pos: IndexedSet<I, P>,
    // Map from the player nodes' ids to the global ids.
    pub ids: IndexedVec<I, NodeId>,
    // Remaining moves for each node.
    pub moves: IndexedVec<I, M>,
    // Set of predecessors for each node.
    pub preds: IndexedVec<I, Set<O>>,
    // Set of successors for each node.
    pub succs: IndexedVec<I, Set<O>>,
    // Set of nodes that can reach unexplored nodes.
    pub escaping: Set<I>,
    // Which player definitely wins on this node
    pub win: IndexedVec<I, WinState>,
    // Set of this player's nodes where player 0 wins
    pub w0: Set<I>,
    // Set of this player's nodes where player 1 wins
    pub w1: Set<I>,
}

impl<I, P, M, O> NodesData<I, P, M, O> {
    pub fn len(&self) -> usize {
        self.pos.len()
    }
}

pub struct Game {
    // Formulas representing the equations in the system.
    pub formulas: EqsFormulas,
    // Data for player 0 nodes.
    pub p0: NodesData<NodeP0Id, P0Pos, P0Moves, NodeP1Id>,
    // Data for player 1 nodes.
    pub p1: NodesData<NodeP1Id, P1Pos, P1Moves, NodeP0Id>,
    // Map between node ids (assumed to also be sorted according to NodeId)
    pub nodes: IndexedVec<NodeId, NodeKind>,
    // Player 0 nodes grouped by VarId, used for sorting by reward.
    // Each inner vec is assumed to be sorted by NodeId.
    pub var_to_p0: IndexedVec<VarId, Vec<NodeP0Id>>,
}

impl Game {
    pub fn new(b: BasisElemId, i: VarId, formulas: EqsFormulas) -> Self {
        let var_count = formulas.var_count();
        let mut game = Self {
            formulas,
            p0: NodesData::default(),
            p1: NodesData::default(),
            nodes: IndexedVec::from(vec![NodeKind::W0, NodeKind::L0, NodeKind::W1, NodeKind::L1]),
            var_to_p0: IndexedVec::from(vec![Vec::new(); var_count]),
        };

        game.insert_p0(P0Pos { b, i });

        game
    }

    pub fn resolve(&self, n: NodeId) -> NodeKind {
        self.nodes[n]
    }

    pub fn player_of(&self, n: NodeId) -> Player {
        match self.resolve(n) {
            NodeKind::L0 | NodeKind::W0 | NodeKind::P0(_) => Player::P0,
            NodeKind::L1 | NodeKind::W1 | NodeKind::P1(_) => Player::P1,
        }
    }

    pub fn relevance_of(&self, node: NodeId) -> Relevance {
        let priority = match self.resolve(node) {
            // High priority (higher than P0 nodes) in favour of P1
            NodeKind::L0 | NodeKind::W1 => 2 * self.formulas.var_count() + 1,
            // High priority (higher than P0 nodes) in favour of P0
            NodeKind::W0 | NodeKind::L1 => 2 * self.formulas.var_count() + 2,
            // Priority proportional to the equation/variable index, going from 1 to 2 * var_count
            NodeKind::P0(n) => {
                let i = self.p0.pos[n].i;
                let fix_type = self.formulas.eq_fix_types[i];
                2 * i.to_usize() + if fix_type == FixType::Max { 2 } else { 1 }
            }
            // This is irrelevant
            NodeKind::P1(_) => 0,
        };
        Relevance { priority, node }
    }

    pub fn formula_of(&self, n: NodeP0Id) -> &Formula {
        let P0Pos { b, i } = self.p0.pos[n];
        self.formulas.get(b, i)
    }

    pub fn successors_of(&self, n: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        match self.resolve(n) {
            // Successors of special nodes are only other special nodes.
            NodeKind::L0 => Left([NodeId::W1]),
            NodeKind::L1 => Left([NodeId::W0]),
            NodeKind::W0 => Left([NodeId::L1]),
            NodeKind::W1 => Left([NodeId::L0]),
            // Successors of a p0/p1 node are either those recorder
            // or the special nodes if they are definitely winning for some players.
            NodeKind::P0(n) => match self.p0.win[n] {
                WinState::Unknown => Right(Left(self.p0.succs[n].iter().map(|&n| self.p1.ids[n]))),
                WinState::Win0 => Left([NodeId::L1]),
                WinState::Win1 => Left([NodeId::W1]),
            },
            NodeKind::P1(n) => match self.p1.win[n] {
                WinState::Unknown => Right(Right(self.p1.succs[n].iter().map(|&n| self.p0.ids[n]))),
                WinState::Win0 => Left([NodeId::W0]),
                WinState::Win1 => Left([NodeId::L0]),
            },
        }
        .into_iter()
    }

    pub fn predecessors_of(&self, n: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        let map_p0 = |&n| self.p0.ids[n];
        let map_p1 = |&n| self.p1.ids[n];
        match self.resolve(n) {
            // The predecessors of special nodes are other special nodes and the definitely winning/losing nodes.
            NodeKind::L0 => Left(Left(self.p1.w1.iter().map(map_p1).chain([NodeId::W1]))),
            NodeKind::L1 => Left(Right(self.p0.w0.iter().map(map_p0).chain([NodeId::W0]))),
            NodeKind::W0 => Left(Left(self.p1.w0.iter().map(map_p1).chain([NodeId::L1]))),
            NodeKind::W1 => Left(Right(self.p0.w1.iter().map(map_p0).chain([NodeId::L0]))),
            // The predecessors of a p0/p1 node are all those recorded in the game.
            NodeKind::P0(n) => Right(Left(self.p0.preds[n].iter().map(map_p1))),
            NodeKind::P1(n) => Right(Right(self.p1.preds[n].iter().map(map_p0))),
        }
    }

    pub fn nodes_sorted_by_reward(&self) -> impl Iterator<Item = NodeId> + '_ {
        let iter = |fix_type| {
            self.var_to_p0
                .enumerate()
                .filter(move |&(i, _)| self.formulas.eq_fix_types[i] == fix_type)
                .flat_map(|(_, nodes)| nodes)
                .map(|&n0| self.p0.ids[n0])
        };

        // Both have odd 2 * var_count + 1 relevance
        let w1_nodes = [NodeId::W1, NodeId::L0].into_iter();
        // These have odd >= 1 relevance and are sorted by decreasing node id.
        let p0_f1_nodes = iter(FixType::Min).rev();
        // These have 0 reward.
        let p1_nodes = self.p1.ids.iter().copied();
        // These have even >=2 reward and are sorted by node id.
        let p0_f0_nodes = iter(FixType::Max);
        // These have 2 * var_count + 2 reward
        let w0_nodes = [NodeId::W0, NodeId::L1].into_iter();

        w1_nodes.chain(p0_f1_nodes).chain(p1_nodes).chain(p0_f0_nodes).chain(w0_nodes)
    }

    /// Inserts a p0 node given its predecessors, updating the sets of predecessors/successors
    /// Returns the id of the node and whether it already existed or not.
    pub fn insert_p0(&mut self, pos: P0Pos) -> Inserted<NodeP0Id> {
        let (n, is_new) = self.p0.pos.insert_full(pos);

        if !is_new {
            return Inserted::Existing(n);
        }

        // If the node is new we need to setup its slot in the various IndexVecs
        self.p0.ids.push(self.nodes.push(NodeKind::P0(n)));
        self.p0.moves.push(pos.moves(&self.formulas));
        self.p0.preds.push(Set::new());
        self.p0.succs.push(Set::new());
        self.p0.escaping.insert(n);
        self.p0.win.push(WinState::Unknown);

        self.var_to_p0[pos.i].push(n);

        Inserted::New(n)
    }

    /// Inserts a p1 node given its predecessors, updating the sets of predecessors/successors
    /// Returns the id of the node and whether it already existed or not.
    pub fn insert_p1(&mut self, pos: P1Pos) -> Inserted<NodeP1Id> {
        let (n, is_new) = self.p1.pos.insert_full(pos.clone());

        if !is_new {
            return Inserted::Existing(n);
        }

        self.p1.ids.push(self.nodes.push(NodeKind::P1(n)));
        self.p1.moves.push(pos.moves());
        self.p1.preds.push(Set::new());
        self.p1.succs.push(Set::new());
        self.p1.escaping.insert(n);
        self.p1.win.push(WinState::Unknown);

        Inserted::New(n)
    }

    pub fn insert_p1_to_p0_edge(&mut self, pred: NodeP1Id, succ: NodeP0Id) {
        self.p0.preds[succ].insert(pred);
        self.p1.succs[pred].insert(succ);
    }

    pub fn insert_p0_to_p1_edge(&mut self, pred: NodeP0Id, succ: NodeP1Id) {
        self.p1.preds[succ].insert(pred);
        self.p0.succs[pred].insert(succ);
    }
}

pub enum Inserted<I> {
    New(I),
    Existing(I),
}

impl<I: Copy> Inserted<I> {
    pub fn id(&self) -> I {
        match *self {
            Self::New(i) | Self::Existing(i) => i,
        }
    }
}

pub struct GameStrategy {
    // The successor is NodeP1Id::INVALID if it's actually W1
    pub direct: IndexedVec<NodeP0Id, NodeP1Id>,
    pub inverse: IndexedVec<NodeP1Id, Set<NodeP0Id>>,
    pub inverse_w1: Set<NodeP0Id>,
    pub inverse_l1: Set<NodeP0Id>,
}

impl GameStrategy {
    pub fn new() -> Self {
        Self {
            direct: IndexedVec::new(),
            inverse: IndexedVec::new(),
            inverse_w1: Set::new(),
            inverse_l1: Set::new(),
        }
    }

    pub fn expand(&mut self, game: &Game) {
        // Ensure the size of inverse is correct.
        self.inverse.resize_with(game.p1.len(), Set::new);

        // Select initial strategy by picking a random successor for each p0 node.
        // Also skip nodes for which the strategy was already initialized.
        for (p0, succs) in game.p0.succs.enumerate().skip(self.direct.len()) {
            let target = succs.first().copied();
            self.direct.push(target.unwrap_or(NodeP1Id::W1));
            match target {
                Some(p1) => _ = self.inverse[p1].insert(p0),
                None => _ = self.inverse_w1.insert(p0),
            }
        }
    }

    pub fn update(&mut self, p0: NodeP0Id, p1: NodeP1Id) {
        let op1 = self.direct[p0];
        match op1 {
            NodeP1Id::W1 => _ = self.inverse_w1.remove(&p0),
            NodeP1Id::L1 => _ = self.inverse_l1.remove(&p0),
            op1 => _ = self.inverse[op1].remove(&p0),
        }

        self.direct[p0] = p1;
        match p1 {
            NodeP1Id::W1 => _ = self.inverse_w1.insert(p0),
            NodeP1Id::L1 => _ = self.inverse_l1.insert(p0),
            p1 => _ = self.inverse[p1].insert(p0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Player {
    P0,
    P1,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Relevance {
    // Actual priority
    pub priority: usize,
    // Used as tiebreaker
    pub node: NodeId,
}

impl Relevance {
    pub fn player(self) -> Player {
        match self.priority % 2 {
            0 => Player::P0,
            _ => Player::P1,
        }
    }

    pub fn reward(self) -> Reward {
        match self.player() {
            Player::P0 => Reward::P0(self),
            Player::P1 => Reward::P1(Reverse(self)),
        }
    }
}

// Note: order is important here. Reward in favour of P1 are considered less
// than rewards in favour of P0. Also, relevance for P1 rewards are considered
// reversed (bigger relevance is worse for P0, and thus less).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reward {
    P1(Reverse<Relevance>),
    Neutral,
    P0(Relevance),
}

impl<I, P, M, O> Default for NodesData<I, P, M, O> {
    fn default() -> Self {
        Self {
            pos: Default::default(),
            ids: Default::default(),
            moves: Default::default(),
            preds: Default::default(),
            succs: Default::default(),
            escaping: Default::default(),
            win: Default::default(),
            w0: Default::default(),
            w1: Default::default(),
        }
    }
}
