use crate::new_index;

new_index!(pub index NodeId);

impl NodeId {
    pub const INVALID: NodeId = NodeId(usize::MAX);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Player {
    P0,
    P1,
}

impl Player {
    pub fn opponent(self) -> Player {
        match self {
            Player::P0 => Player::P1,
            Player::P1 => Player::P0,
        }
    }
}

pub trait Solver {
    fn new() -> Self;

    fn add_node(&mut self, player: Player, priority: usize) -> NodeId;
    fn add_edge(&mut self, u: NodeId, v: NodeId);
    fn remove_edge(&mut self, u: NodeId, v: NodeId);
    fn remove_node(&mut self, n: NodeId);

    fn predecessors(&self, n: NodeId) -> impl Iterator<Item = NodeId> + '_;
    fn player(&self, n: NodeId) -> Player;

    fn solve(&mut self);
    fn winner(&self, n: NodeId) -> Player;

    fn inverse_strategy(&self, n: NodeId) -> impl Iterator<Item = NodeId> + '_;
}
