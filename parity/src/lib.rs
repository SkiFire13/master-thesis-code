mod conv;
mod parser;

#[cfg(test)]
mod test;

pub use conv::parity_game_to_fix;
pub use parser::parse_parity_game;
use solver::strategy::Player;

#[derive(Debug)]
pub struct Node {
    pub id: usize,
    pub relevance: usize,
    pub player: Player,
    pub successors: Vec<usize>,
}

#[derive(Debug)]
pub struct ParityGame {
    pub nodes: Vec<Node>,
}
