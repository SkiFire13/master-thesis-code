mod escape;
mod expand;

use std::collections::HashMap;
use std::hash::Hash;

use crate::index::{IndexedSet, IndexedVec};
use crate::solver::{NodeId, Player, Solver};
use crate::{new_index, Set};

#[derive(PartialEq, Eq)]
pub enum WinState {
    Win(Player),
    Unknown,
}

pub trait Simplify<T> {
    fn simplify(&mut self, winning: impl Fn(&T) -> WinState);
}

pub trait Expander {
    type Pos: Hash + Eq;
    type Moves: Iterator<Item = Self::Pos> + Simplify<Self::Pos>;

    fn player(&self, pos: &Self::Pos) -> Player;
    fn priority(&self, pos: &Self::Pos) -> usize;

    fn moves_for(&mut self, pos: &Self::Pos) -> Self::Moves;
}

pub struct LocalSolver<S, E: Expander> {
    expander: E,
    pos: IndexedSet<LNodeId, E::Pos>,
    moves: IndexedVec<LNodeId, E::Moves>,

    solver: S,
    nodes: IndexedVec<LNodeId, NodeId>,
    node_to_id: rustc_hash::FxHashMap<NodeId, LNodeId>,

    win: IndexedVec<LNodeId, Option<Player>>,
    boundary_p0: Set<LNodeId>,
    boundary_p1: Set<LNodeId>,
    explore_goal: usize,
}

new_index!(index LNodeId);

#[derive(Clone, Copy)]
enum Inserted<I> {
    New(I),
    Existing(I),
}

impl<I> Inserted<I> {
    fn id(self) -> I {
        match self {
            Inserted::New(id) => id,
            Inserted::Existing(id) => id,
        }
    }
}

impl<S: Solver, E: Expander> LocalSolver<S, E> {
    pub fn new(expander: E) -> Self {
        Self {
            expander,
            pos: IndexedSet::default(),
            nodes: IndexedVec::default(),
            win: IndexedVec::new(),

            solver: S::new(),
            moves: IndexedVec::new(),
            node_to_id: HashMap::default(),

            boundary_p0: Set::default(),
            boundary_p1: Set::default(),
            explore_goal: 1,
        }
    }

    fn insert(&mut self, pos: E::Pos) -> Inserted<LNodeId> {
        let (id, is_new) = self.pos.insert_full(pos);

        if !is_new {
            return Inserted::Existing(id);
        }

        // TODO: simplify moves?
        let moves = self.expander.moves_for(&self.pos[id]);
        let player = self.expander.player(&self.pos[id]);
        let priority = self.expander.priority(&self.pos[id]);

        self.moves.push(moves);
        self.win.push(None);

        let node = self.solver.add_node(player, priority);
        self.nodes.push(node);
        self.node_to_id.insert(node, id);

        match player {
            Player::P0 => _ = self.boundary_p0.insert(id),
            Player::P1 => _ = self.boundary_p1.insert(id),
        }

        Inserted::New(id)
    }

    pub fn solve(&mut self, pos: E::Pos) -> Player {
        let init = self.insert(pos).id();

        loop {
            self.expand(init);
            self.solver.solve();
            self.update_winning_sets();

            if let Some(winner) = self.win[init] {
                return winner;
            }
        }
    }
}
