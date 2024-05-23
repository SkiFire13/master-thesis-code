use std::collections::HashSet;

use crate::index::IndexVec;
use crate::strategy::game::{NodeId, Player};

use super::game::{Game, NodeKind};
use super::improvement::PlayProfile;

pub fn expand(game: &mut Game, profiles: &IndexVec<NodeId, PlayProfile>) {
    let mut a = e1(game, profiles);
    let mut new_a = Vec::new();
    let mut seen = HashSet::new();

    while !a.is_empty() {
        for v in a.drain(..) {
            e2(game, v, profiles, |n| {
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

fn e2(
    game: &mut Game,
    w: NodeKind,
    profiles: &IndexVec<NodeId, PlayProfile>,
    mut add: impl FnMut(NodeKind),
) {
    match w {
        NodeKind::W0 | NodeKind::L0 | NodeKind::W1 | NodeKind::L1 => unreachable!(),
        NodeKind::P0(n) => {
            // TODO: apply decisions and maybe assumptions?
            _ = profiles;

            let mov = match game.formula_of(n).next_move() {
                Some(mov) => mov,
                None => {
                    // The formula is false so the successor is W1
                    game.w1_preds.push(n);
                    return;
                }
            };

            let (p1, is_new) = game.insert_p1(n, mov);
            if is_new {
                add(NodeKind::P1(p1))
            }
        }
        NodeKind::P1(n) => {
            // TODO: Skip already explored nodes?
            for &bi in &*game.p1_set[n].clone() {
                let (p0, is_new) = game.insert_p0(n, bi);
                if is_new {
                    add(NodeKind::P0(p0));
                }
            }
        }
    }
}
