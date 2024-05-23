use std::collections::HashSet;

use crate::index::IndexedVec;
use crate::strategy::game::{NodeId, Player};

use super::game::{Game, NodeKind, WinState};
use super::improvement::PlayProfile;

pub fn expand(game: &mut Game, profiles: &IndexedVec<NodeId, PlayProfile>) {
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

fn e1(game: &mut Game, profiles: &IndexedVec<NodeId, PlayProfile>) -> Vec<NodeKind> {
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
    profiles: &IndexedVec<NodeId, PlayProfile>,
    mut add: impl FnMut(NodeKind),
) {
    match w {
        NodeKind::W0 | NodeKind::L0 | NodeKind::W1 | NodeKind::L1 => unreachable!(),
        NodeKind::P0(n) => {
            // TODO: apply decisions and maybe assumptions?
            _ = profiles;

            let f = game.formula_of(n);
            if f.is_false() {
                // The formula is false so the successor is W1
                game.p0.win[n] = WinState::Win1;
                game.p0.w1.push(n);
                return;
            }

            // TODO: This doesn't skip already explored nodes.
            let mov = match game.formula_of(n).next_move() {
                Some(mov) => mov,
                None => {
                    // TODO: Set node as non-escaping.
                    return;
                }
            };

            let (p1, is_new) = game.insert_p1(n, mov);
            if is_new {
                add(NodeKind::P1(p1))
            }
        }
        NodeKind::P1(n) => {
            // The node has no move at all, so its only successor is W0, add it to that set.
            if game.p1.data[n].is_empty() {
                game.p1.win[n] = WinState::Win0;
                game.p1.w0.push(n);
                return;
            }

            // TODO: This doesn't skip already explored nodes.
            // TODO: Set node as non-escaping when it has already visited all successors.
            for &bi in &*game.p1.data[n].clone() {
                let (p0, is_new) = game.insert_p0(n, bi);
                if is_new {
                    add(NodeKind::P0(p0));
                }
            }
        }
    }
}
