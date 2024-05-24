use std::cmp::Reverse;
use std::rc::Rc;

use either::Either::{Left, Right};

use crate::index::{new_index, AsIndex, IndexedSet, IndexedVec};
use crate::symbolic::compose::EqsFormulas;
use crate::symbolic::eq::{FixType, VarId};
use crate::symbolic::formula::{BasisElemId, Formula};

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

pub enum WinState {
    Unknown,
    Win0,
    Win1,
}

// Group of informations for a player nodes
pub struct NodesData<I: AsIndex, T, O> {
    // Data representing a node. Also used for deduping and mapping it to a numeric id.
    pub data: IndexedSet<I, T>,
    // Map from the player nodes' ids to the global ids.
    pub node_ids: IndexedVec<I, NodeId>,
    // Set of predecessors for each node.
    pub preds: IndexedVec<I, Set<O>>,
    // Set of successors for each node.
    pub succs: IndexedVec<I, Set<O>>,
    // Which player definitely wins on this node
    pub win: IndexedVec<I, WinState>,
    // List of this player's nodes where player 0 wins
    pub w0: Vec<I>,
    // List of this player's nodes where player 1 wins
    pub w1: Vec<I>,
}

pub struct Game {
    // Formulas representing the equations in the system.
    pub formulas: EqsFormulas,
    // Data for player 0 nodes.
    pub p0: NodesData<NodeP0Id, (BasisElemId, VarId), NodeP1Id>,
    // Data for player 1 nodes.
    pub p1: NodesData<NodeP1Id, Rc<[(BasisElemId, VarId)]>, NodeP0Id>,
    // Map between node ids (assumed to also be sorted according to NodeId)
    pub nodes: IndexedVec<NodeId, NodeKind>,
    // Player 0 nodes grouped by VarId, used for sorting by reward.
    // Each inner vec is assumed to be sorted by NodeId.
    pub var_to_p0: IndexedVec<VarId, Vec<NodeP0Id>>,
    // Set of nodes that can escape.
    pub escaping: Set<NodeId>,
}

impl Game {
    pub fn new(b: BasisElemId, i: VarId, formulas: EqsFormulas) -> Self {
        let mut var_to_p0 = IndexedVec::from(vec![Vec::new(); formulas.var_count()]);
        var_to_p0[i].push(NodeP0Id::INIT);

        Self {
            formulas,

            // p0 initially contains only (b, i) and related data
            p0: NodesData {
                data: IndexedSet::from([(b, i)]),
                node_ids: IndexedVec::from([NodeId::INIT]),
                preds: IndexedVec::from([Set::new()]),
                succs: IndexedVec::from([Set::new()]),
                win: IndexedVec::from([WinState::Unknown]),
                w0: Vec::new(),
                w1: Vec::new(),
            },

            // p1 initially is empty
            p1: NodesData {
                data: IndexedSet::new(),
                node_ids: IndexedVec::new(),
                preds: IndexedVec::new(),
                succs: IndexedVec::new(),
                win: IndexedVec::new(),
                w0: Vec::new(),
                w1: Vec::new(),
            },

            // Nodes contains the extra dummy nodes and the initial (b, i)
            nodes: IndexedVec::from(vec![
                NodeKind::W0,
                NodeKind::L0,
                NodeKind::W1,
                NodeKind::L1,
                NodeKind::P0(NodeP0Id::INIT),
            ]),

            var_to_p0,

            escaping: Set::new(),
        }
    }

    pub fn resolve(&self, n: NodeId) -> NodeKind {
        self.nodes[n]
    }

    pub fn relevance_of(&self, n: NodeId) -> Relevance {
        let rel = match self.resolve(n) {
            // High relevance (higher than P0 nodes) in favour of P1
            NodeKind::L0 | NodeKind::W1 => 2 * self.formulas.var_count() + 1,
            // High relevance (higher than P0 nodes) in favour of P0
            NodeKind::W0 | NodeKind::L1 => 0,
            // Relevance proportional to the variable index, going from 1 to 2 * var_count
            NodeKind::P0(n) => {
                let (_, i) = self.p0.data[n];
                let fix_type = self.formulas.eq_fix_types[i];
                // TODO: Maybe optimize this to make it more compact?
                // TODO: Is this Min vs Max correct?
                2 * i.to_usize() + if fix_type == FixType::Max { 2 } else { 1 }
            }
            // This is irrelevant
            NodeKind::P1(_) => 0,
        };
        Relevance(rel, n)
    }

    pub fn formula_of(&self, n: NodeP0Id) -> &Formula {
        let (b, i) = self.p0.data[n];
        self.formulas.get(b, i)
    }

    pub fn successors_of(&self, n: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        match self.resolve(n) {
            // Successors of special nodes are only other special nodes.
            NodeKind::L0 => Left([NodeId::W1]),
            NodeKind::L1 => Left([NodeId::W0]),
            NodeKind::W0 => Left([NodeId::L1]),
            NodeKind::W1 => Left([NodeId::L0]),
            // The successors of a p0/p1 node are all those recorded in the node data.
            NodeKind::P0(n) => Right(Left(self.p0.succs[n].iter().map(|&n| self.p1.node_ids[n]))),
            NodeKind::P1(n) => Right(Right(self.p1.succs[n].iter().map(|&n| self.p0.node_ids[n]))),
        }
        .into_iter()
    }

    pub fn predecessors_of(&self, n: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        match self.resolve(n) {
            // The predecessor of a L node is just the corresponding W node.
            NodeKind::L0 => Left(&[NodeId::W1][..]),
            NodeKind::L1 => Left(&[NodeId::W0][..]),
            // The predecessor of the W0 node is the empty P1 node
            NodeKind::W0 => Right(Left(Left(self.p1.w0.iter()))),
            // The predecessor of the W1 node are all those P0 nodes with a false formula.
            NodeKind::W1 => Right(Right(Left(self.p0.w1.iter()))),
            // The predecessors of a p0/p1 node are all those recorded in the game.
            NodeKind::P0(n) => Right(Left(Right(self.p0.preds[n].iter()))),
            NodeKind::P1(n) => Right(Right(Right(self.p1.preds[n].iter()))),
        }
        .map_left(|slice| slice.iter().copied())
        .map_right(|inner| inner.map_left(|iter| iter.map(|&n| self.p1.node_ids[n])))
        .map_right(|inner| inner.map_right(|iter| iter.map(|&n| self.p0.node_ids[n])))
    }

    pub fn nodes_sorted_by_reward(&self) -> impl Iterator<Item = NodeId> + '_ {
        let iter = |fix_type| {
            self.var_to_p0
                .enumerate()
                .filter(move |&(i, _)| self.formulas.eq_fix_types[i] == fix_type)
                .flat_map(|(_, nodes)| nodes)
                .map(|&n0| self.p0.node_ids[n0])
        };

        // Both have 2 * var_count + 1 relevance and low node id
        let w1_nodes = [NodeId::W1, NodeId::L0].into_iter();
        // These has <=-1 reward and high node id
        let p0_f1_nodes = iter(FixType::Min).rev();
        // These have 0 reward and lower node id than others
        let w0_nodes = [NodeId::W0, NodeId::L1].into_iter();
        // These have 0 reward and bigger node id than W/L nodes
        let p1_nodes = self.p1.node_ids.iter().copied();
        // These have >=2 reward and are sorted by node id.
        let p0_f0_nodes = iter(FixType::Max);

        w1_nodes
            .chain(p0_f1_nodes)
            .chain(w0_nodes)
            .chain(p1_nodes)
            .chain(p0_f0_nodes)
    }

    /// Inserts a p0 node given its predecessors, updating the sets of predecessors/successors
    /// Returns the id of the node and whether it already existed or not.
    pub fn insert_p0(&mut self, pred: NodeP1Id, node: (BasisElemId, VarId)) -> (NodeP0Id, bool) {
        let (idx, is_new) = self.p0.data.insert_full(node);
        let p0id = NodeP0Id::from_usize(idx);

        // If the node is new we need to setup its slot in the various IndexVecs
        if is_new {
            let nid = self.nodes.push(NodeKind::P0(p0id));
            self.p0.node_ids.push(nid);

            self.p0.preds.push(Set::new());
            self.p0.succs.push(Set::new());

            let (_, i) = node;
            self.var_to_p0[i].push(p0id);

            self.escaping.insert(nid);
        }

        // Always set predecessors/successors
        self.p0.preds[p0id].insert(pred);
        self.p1.succs[pred].insert(p0id);

        (p0id, is_new)
    }

    /// Inserts a p1 node given its predecessors, updating the sets of predecessors/successors
    /// Returns the id of the node and whether it already existed or not.
    pub fn insert_p1(
        &mut self,
        pred: NodeP0Id,
        node: Rc<[(BasisElemId, VarId)]>,
    ) -> (NodeP1Id, bool) {
        let (idx, is_new) = self.p1.data.insert_full(node);
        let p1id = NodeP1Id::from_usize(idx);

        // If the node is new we need to setup its slot in the various IndexVecs
        if is_new {
            let nid = self.nodes.push(NodeKind::P1(p1id));
            self.p1.node_ids.push(nid);

            self.p1.preds.push(Set::new());
            self.p1.succs.push(Set::new());

            self.escaping.insert(nid);
        }

        // Always set predecessors/successors
        self.p0.succs[pred].insert(p1id);
        self.p1.preds[p1id].insert(pred);

        (p1id, is_new)
    }
}

pub struct GameStrategy {
    // The successor is None if it's actually W1
    pub direct: IndexedVec<NodeP0Id, Option<NodeP1Id>>,
    pub inverse: IndexedVec<NodeP1Id, Set<NodeP0Id>>,
    pub inverse_w1: Set<NodeP0Id>,
}

impl GameStrategy {
    pub fn new() -> Self {
        Self {
            direct: IndexedVec::new(),
            inverse: IndexedVec::new(),
            inverse_w1: Set::new(),
        }
    }

    pub fn expand(&mut self, game: &Game) {
        // Ensure the size of inverse is correct.
        self.inverse.resize_with(game.p1.data.len(), Set::new);

        // Select initial strategy by picking a random successor for each p0 node.
        // Also skip nodes for which the strategy was already initialized.
        for (p0, succs) in game.p0.succs.enumerate().skip(self.direct.len()) {
            let target = succs.first().copied();
            self.direct.push(target);
            match target {
                Some(p1) => _ = self.inverse[p1].insert(p0),
                None => _ = self.inverse_w1.insert(p0),
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Player {
    P0,
    P1,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Relevance(pub usize, pub NodeId);

impl Relevance {
    pub fn player(self) -> Player {
        match self.0 % 2 {
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
