use std::collections::HashSet;

use crate::index::IndexedVec;
use crate::strategy::game::{NodeId, Player};

use super::game::{Game, Inserted, NodeKind, WinState};
use super::improvement::PlayProfile;

pub fn expand(game: &mut Game, profiles: &IndexedVec<NodeId, PlayProfile>) {
    let mut a = e1(game, profiles);
    let mut new_a = Vec::new();
    let mut seen = HashSet::new();

    while !a.is_empty() {
        for v in a.drain(..) {
            e2(game, v, profiles, |n| {
                debug_assert!(seen.insert(n));
                new_a.push(n);
            });
        }

        std::mem::swap(&mut a, &mut new_a);
    }
}

fn e1(game: &mut Game, profiles: &IndexedVec<NodeId, PlayProfile>) -> Vec<NodeId> {
    // TODO: This should select an element of the exterior, not one that can reach the exterior.
    // In practice this shouldn't matter though.
    match profiles[NodeId::INIT].winning(game) {
        Player::P0 => {
            let p1 = game.p1.escaping.first().copied().unwrap();
            let n = game.p1.ids[p1];
            vec![n]
        }
        Player::P1 => {
            let p0 = game.p0.escaping.first().copied().unwrap();
            let n = game.p0.ids[p0];
            vec![n]
        }
    }
}

fn e2(
    game: &mut Game,
    w: NodeId,
    profiles: &IndexedVec<NodeId, PlayProfile>,
    mut add: impl FnMut(NodeId),
) {
    match game.resolve(w) {
        NodeKind::W0 | NodeKind::L0 | NodeKind::W1 | NodeKind::L1 => unreachable!(),
        NodeKind::P0(n) => {
            if game.formula_of(n).is_false() {
                // The formula is false so the successor is W1
                // and the node is winning for p1.
                game.p0.win[n] = WinState::Win1;
                game.p0.w1.insert(n);
                game.p0.escaping.remove(&n);
                return;
            }

            // TODO: use profiles to avoid non-improving moves.
            _ = profiles;

            let Some(pos) = game.p0.moves[n].next() else {
                game.p0.escaping.remove(&n);
                return;
            };

            let inserted = game.insert_p1(pos);
            game.insert_p0_to_p1_edge(n, inserted.id());

            if let Inserted::New(p1) = inserted {
                add(game.p1.ids[p1])
            }
        }
        NodeKind::P1(n) => {
            // The node has no move at all, so its only successor is W0
            // and the node is winning for p0.
            if game.p1.pos[n].moves.is_empty() {
                game.p1.win[n] = WinState::Win0;
                game.p1.w0.insert(n);
                game.p1.escaping.remove(&n);
                return;
            }

            // TODO: Make this an input setting?
            // Toggles symmetric and asymmetric algorithm
            const SYMMETRIC: bool = true;

            if SYMMETRIC {
                // Symmetric version: consider next position
                let Some(pos) = game.p1.moves[n].next() else {
                    game.p1.escaping.remove(&n);
                    return;
                };

                let inserted = game.insert_p0(pos);
                game.insert_p1_to_p0_edge(n, inserted.id());

                if let Inserted::New(p0) = inserted {
                    add(game.p0.ids[p0]);
                }
            } else {
                // Asymmetric version: iterate over all remaining moves
                for pos in std::mem::take(&mut game.p1.moves[n]) {
                    let inserted = game.insert_p0(pos);
                    game.insert_p1_to_p0_edge(n, inserted.id());

                    if let Inserted::New(p0) = inserted {
                        add(game.p0.ids[p0]);
                    }
                }

                game.p1.escaping.remove(&n);
            }
        }
    }
}
