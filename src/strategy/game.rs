use std::cmp::Reverse;

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

    // Set of nodes, to given them an identity (the index in the set)
    pub p0_set: IndexSet<NodeP0Id, (BasisId, VarId)>,
    pub p1_set: IndexSet<NodeP1Id, Vec<(BasisId, VarId)>>,

    // Map between node ids
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
}

impl Game {
    pub fn new(b: BasisId, i: VarId, formulas: EqsFormulas) -> Self {
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
                2 * i.to_usize() + if let FixType::Min = fix_type { 0 } else { 1 }
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
}

pub enum Player {
    P0,
    P1,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Relevance(usize, NodeId);

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
    P0(Relevance),
}
