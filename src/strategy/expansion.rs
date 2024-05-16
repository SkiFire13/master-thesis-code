use std::collections::HashSet;

use crate::index::IndexVec;
use crate::strategy::game::{NodeId, NodeP0Id, Player};

use super::game::{Game, NodeKind};
use super::improvement::PlayProfile;

pub fn expand(game: &mut Game, profiles: &IndexVec<NodeId, PlayProfile>) {
    let mut a = e1(game, profiles);
    let mut new_a = Vec::new();
    let mut seen = HashSet::new();

    while !a.is_empty() {
        game.nodes.extend(&a);

        for v in a.drain(..) {
            e2(game, v, |n| {
                // Make new_a unique
                if seen.insert(n) {
                    new_a.push(n);
                }
            });
        }

        std::mem::swap(&mut a, &mut new_a);
    }
}

fn e1(game: &mut Game, profiles: &IndexVec<NodeId, PlayProfile>) -> Vec<NodeKind> {
    // TODO: Is this correct?
    let init_node = NodeId::INIT;
    let relevant_node = profiles[init_node].most_relevant;

    match game.relevance_of(relevant_node).player() {
        Player::P0 => {
            // TODO: Find unexplored node from p1 and expand it
            // (Bonus: reachable from current strategy?)
            todo!();
        }
        Player::P1 => {
            // TODO: Find unexplored node from p0 and expand it
            // (Bonus: reachable from current strategy?)

            // TODO: also permanently apply decisions
            todo!();
        }
    }
}

fn e2(game: &mut Game, w: NodeKind, mut add: impl FnMut(NodeKind)) {
    match w {
        // TODO: will these ever be hit?
        NodeKind::W0 => add(NodeKind::L1),
        NodeKind::L0 => add(NodeKind::W1),
        NodeKind::W1 => add(NodeKind::L0),
        NodeKind::L1 => add(NodeKind::W0),
        NodeKind::P0(n) if game.formula_of(n).is_false() => add(NodeKind::W1),
        NodeKind::P1(n) if game.p1_set[n].is_empty() => add(NodeKind::W0),
        NodeKind::P0(n) => {
            // TODO: apply decisions and stuff
            todo!();
        }
        NodeKind::P1(n) => {
            for &bi in &game.p1_set[n] {
                if game.p0_set.get(&bi).is_none() {
                    // TODO: Better add node, update succ/pred etc etc
                    let (idx, _) = game.p0_set.insert_full(bi);
                    add(NodeKind::P0(NodeP0Id(idx)))
                    // TODO: Add forward and backward edges

                    // Only in synchronous version:
                    // break;
                }
            }
        }
    }

    todo!()
}
