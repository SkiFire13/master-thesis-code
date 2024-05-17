use std::cmp::Reverse;
use std::slice;

use either::Either::{Left, Right};

use crate::index::{new_index, AsIndex, IndexSet, IndexVec};
use crate::symbolic::compose::EqsFormulas;
use crate::symbolic::eq::{FixType, VarId};
use crate::symbolic::formula::{BasisId, Formula};

new_index!(pub index NodeId);

impl NodeId {
    pub const W0: NodeId = NodeId(0);
    pub const L0: NodeId = NodeId(1);
    pub const W1: NodeId = NodeId(2);
    pub const L1: NodeId = NodeId(3);

    pub const INIT: NodeId = NodeId(4);
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
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

pub struct Game {
    pub formulas: EqsFormulas,

    // Set of nodes, to give them an identity (the index in their set)
    pub p0_set: IndexSet<NodeP0Id, (BasisId, VarId)>,
    pub p1_set: IndexSet<NodeP1Id, Vec<(BasisId, VarId)>>,

    // Map between node ids (assumed to also be sorted according to NodeId)
    pub nodes: IndexVec<NodeId, NodeKind>,
    pub p0_ids: IndexVec<NodeP0Id, NodeId>,
    pub p1_ids: IndexVec<NodeP1Id, NodeId>,

    // Predecessors of each node type
    pub p0_preds: IndexVec<NodeP0Id, Vec<NodeP1Id>>,
    pub p1_preds: IndexVec<NodeP1Id, Vec<NodeP0Id>>,
    pub w1_preds: Vec<NodeP0Id>,
    // Successors of each node type
    pub p0_succs: IndexVec<NodeP0Id, Vec<NodeP1Id>>,
    pub p1_succs: IndexVec<NodeP1Id, Vec<NodeP0Id>>,

    // Player 0 nodes grouped by VarId, used for sorting by reward.
    // Each inner vec is assumed to be sorted by NodeId.
    pub p0_by_var: IndexVec<VarId, Vec<NodeP0Id>>,
    // TODO: w0: Set<NodeId>,
    // TODO: w1: Set<NodeId>,
    // TODO: support for incrementally extending w0 and w1
}

impl Game {
    pub fn new(b: BasisId, i: VarId, formulas: EqsFormulas) -> Self {
        let mut p0_by_var = IndexVec::from(vec![Vec::new(); formulas.var_count()]);
        p0_by_var[i].push(NodeP0Id(0));

        Self {
            formulas,

            p0_set: IndexSet::from([(b, i)]),
            p1_set: IndexSet::new(),

            nodes: IndexVec::from(vec![
                NodeKind::W0,
                NodeKind::L0,
                NodeKind::W1,
                NodeKind::L1,
                NodeKind::P0(NodeP0Id(0)),
            ]),
            p0_ids: IndexVec::from(vec![NodeId::INIT]),
            p1_ids: IndexVec::new(),

            p0_preds: IndexVec::new(),
            p1_preds: IndexVec::new(),
            w1_preds: Vec::new(),
            p0_succs: IndexVec::new(),
            p1_succs: IndexVec::new(),

            p0_by_var,
        }
    }

    pub fn resolve(&self, n: NodeId) -> NodeKind {
        self.nodes[n]
    }

    pub fn relevance_of(&self, n: NodeId) -> Relevance {
        let rel = match self.resolve(n) {
            NodeKind::L0 => 1,
            NodeKind::L1 => 0,
            NodeKind::W0 => 0,
            NodeKind::W1 => 1,
            NodeKind::P0(n) => {
                let (_, i) = self.p0_set[n];
                let fix_type = self.formulas.eq_fix_types[i];
                // TODO: Maybe optimize this to make it more compact?
                2 * i.to_usize() + if fix_type == FixType::Min { 2 } else { 1 }
            }
            NodeKind::P1(_) => 0,
        };
        Relevance(rel, n)
    }

    pub fn formula_of(&self, n: NodeP0Id) -> &Formula {
        let (b, i) = self.p0_set[n];
        self.formulas.get(b, i)
    }

    pub fn w0_pred(&self) -> Option<NodeP1Id> {
        self.p1_set.get_index_of(&Vec::new()).map(NodeP1Id)
    }

    pub fn successors_of(&self, n: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        match self.resolve(n) {
            // Successors of special nodes are only other special nodes.
            NodeKind::L0 => Left(&[NodeId::W1][..]),
            NodeKind::L1 => Left(&[NodeId::W0][..]),
            NodeKind::W0 => Left(&[NodeId::L1][..]),
            NodeKind::W1 => Left(&[NodeId::L0][..]),
            // The successors of a p0/p1 node are all those recorded in the current game.
            NodeKind::P0(n) => Right(Left(self.p0_succs[n].iter().map(|&n| self.p1_ids[n]))),
            NodeKind::P1(n) => Right(Right(self.p1_succs[n].iter().map(|&n| self.p0_ids[n]))),
        }
        .map_left(|slice| slice.iter().copied())
    }

    pub fn predecessors_of(&self, n: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        match self.resolve(n) {
            // The predecessor of a L node is just the corresponding W node.
            NodeKind::L0 => Left(&[NodeId::W1][..]),
            NodeKind::L1 => Left(&[NodeId::W0][..]),
            // The predecessor of the W0 node is the empty P1 node
            NodeKind::W0 => Left(
                self.w0_pred()
                    .map_or(&[][..], |n| slice::from_ref(&self.p1_ids[n])),
            ),
            // The predecessor of the W1 node are all those P0 nodes with a false formula.
            NodeKind::W1 => Right(Right(self.w1_preds.iter())),
            // The predecessors of a p0/p1 node are all those recorded in the game.
            NodeKind::P0(n) => Right(Left(self.p0_preds[n].iter().map(|&n| self.p1_ids[n]))),
            NodeKind::P1(n) => Right(Right(self.p1_preds[n].iter())),
        }
        .map_left(|slice| slice.iter().copied())
        .map_right(|inner| inner.map_right(|iter| iter.map(|&n| self.p0_ids[n])))
    }

    pub fn nodes_sorted_by_reward(&self) -> impl Iterator<Item = NodeId> + '_ {
        let iter = |fix_type| {
            self.p0_by_var
                .iter()
                .enumerate()
                .filter(move |&(i, _)| self.formulas.eq_fix_types[VarId(i)] == fix_type)
                .flat_map(|(_, nodes)| nodes)
                .map(|&n0| self.p0_ids[n0])
        };

        // These has <=-1 reward and high node id
        let p0_f1_nodes = iter(FixType::Min).rev();
        // These have -1/0 reward and low node id
        let wl_nodes = [NodeId::W1, NodeId::L0, NodeId::W0, NodeId::L1];
        // These have 0 reward and bigger node id than W/L nodes
        let p1_nodes = self.p1_ids.iter().copied();
        // These have >=2 reward and are sorted by node id.
        let p0_f0_nodes = iter(FixType::Max);

        p0_f1_nodes
            .chain(wl_nodes)
            .chain(p1_nodes)
            .chain(p0_f0_nodes)
    }
}

pub struct GameStrategy {
    // TODO: Not all p0 nodes can have an actual successor.
    pub direct: IndexVec<NodeP0Id, NodeP1Id>,
    pub inverse: IndexVec<NodeP1Id, Vec<NodeP0Id>>,
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
