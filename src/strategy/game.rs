use std::cmp::Reverse;

use crate::index::{new_index, AsIndex, IndexSet, IndexVec};
use crate::symbolic::compose::EqsFormulas;
use crate::symbolic::eq::{FixType, VarId};
use crate::symbolic::formula::BasisId;

use super::improvement::PlayProfile;

new_index!(pub index NodeId);

impl NodeId {
    pub const W0: NodeId = NodeId(0);
    pub const L0: NodeId = NodeId(1);
    pub const W1: NodeId = NodeId(2);
    pub const L1: NodeId = NodeId(3);

    pub const INIT: NodeId = NodeId(4);
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeData {
    W0,
    L0,
    W1,
    L1,
    P0(NodeP0Id),
    P1(NodeP1Id),
}

new_index!(pub index NodeP0Id);
new_index!(pub index NodeP1Id);

pub struct Game {
    pub formulas: EqsFormulas,

    pub nodes: IndexVec<NodeId, NodeData>,
    // TODO: informations about the node and in/out edges
    pub nodes_p0: IndexSet<NodeP0Id, (BasisId, VarId)>,
    // TODO: informations about the node and in/out edges
    pub nodes_p1: IndexSet<NodeP1Id, Vec<(BasisId, VarId)>>,

    pub p0_node_ids: IndexVec<NodeP0Id, NodeId>,
    pub p1_node_ids: IndexVec<NodeP1Id, NodeId>,

    // pub successors_p0: Vec<Vec<NodeP1Id>>,
    // pub successors_p1: Vec<Vec<NodeP0Id>>,
    // pub predecessors_p0: Vec<Vec<NodeP1Id>>,
    // pub predecessors_p1: Vec<Vec<NodeP0Id>>,
    // TODO: Something for sorted by reward?
    pub profiles: IndexVec<NodeId, PlayProfile>,
}

impl Game {
    pub fn new(b: BasisId, i: VarId, formulas: EqsFormulas) -> Self {
        Self {
            formulas,
            nodes: IndexVec::from(vec![
                NodeData::W0,
                NodeData::L0,
                NodeData::W1,
                NodeData::L1,
                NodeData::P0(NodeP0Id(0)),
            ]),
            nodes_p0: IndexSet::from([(b, i)]),
            nodes_p1: IndexSet::new(),
            p0_node_ids: IndexVec::from(vec![NodeId::INIT]),
            p1_node_ids: IndexVec::new(),
            // successors_p0: Vec::new(),
            // successors_p1: Vec::new(),
            // predecessors_p0: Vec::new(),
            // predecessors_p1: Vec::new(),
            profiles: IndexVec::new(),
        }
    }

    pub fn resolve(&self, n: NodeId) -> NodeData {
        self.nodes[n]
    }

    pub fn relevance_of(&self, n: NodeId) -> Relevance {
        let rel = match self.resolve(n) {
            NodeData::W0 => 0,
            NodeData::L0 => 1,
            NodeData::W1 => 1,
            NodeData::L1 => 0,
            NodeData::P0(n) => {
                let (_, i) = self.nodes_p0[n];
                let fix_type = self.formulas.eq_fix_types[i];
                // TODO: Maybe optimize this to make it more compact?
                2 * i.to_usize() + if let FixType::Min = fix_type { 0 } else { 1 }
            }
            NodeData::P1(_) => 0,
        };
        Relevance(rel, n)
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
