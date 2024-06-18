use std::cmp::Reverse;

use crate::new_index;

pub trait ParityGraph: GetRelevance {
    fn node_count(&self) -> usize;

    fn player_of(&self, n: NodeId) -> Player;

    fn successors_of(&self, n: NodeId) -> impl Iterator<Item = NodeId>;
    fn predecessors_of(&self, n: NodeId) -> impl Iterator<Item = NodeId>;

    fn nodes_sorted_by_reward(&self) -> impl Iterator<Item = NodeId>;
}

new_index!(pub index NodeId);

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

pub trait GetRelevance {
    fn relevance_of(&self, u: NodeId) -> Relevance;

    fn reward_of(&self, u: NodeId) -> Reward {
        self.relevance_of(u).reward()
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
