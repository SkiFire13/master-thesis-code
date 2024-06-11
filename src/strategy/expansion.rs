use std::collections::HashSet;

use crate::index::IndexedVec;
use crate::strategy::game::{NodeId, Player};

use super::game::{Game, GameStrategy, Inserted, NodeKind, NodeP1Id};
use super::improvement::PlayProfile;

pub fn expand(
    game: &mut Game,
    profiles: &IndexedVec<NodeId, PlayProfile>,
    strategy: &mut GameStrategy,
) {
    let mut a = e1(game, profiles);
    let mut new_a = Vec::new();
    let mut seen = HashSet::new();

    while !a.is_empty() {
        for v in a.drain(..) {
            e2(game, v, profiles, strategy, |n| {
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
            let p1 = game.p1.incomplete.first().copied().unwrap();
            let n = game.p1.ids[p1];
            vec![n]
        }
        Player::P1 => {
            let p0 = game.p0.incomplete.first().copied().unwrap();
            let n = game.p0.ids[p0];
            vec![n]
        }
    }
}

fn e2(
    game: &mut Game,
    w: NodeId,
    profiles: &IndexedVec<NodeId, PlayProfile>,
    strategy: &mut GameStrategy,
    mut add: impl FnMut(NodeId),
) {
    match game.resolve(w) {
        NodeKind::W0 | NodeKind::L0 | NodeKind::W1 | NodeKind::L1 => unreachable!(),
        NodeKind::P0(n) => {
            // Handle case where node never had any moves.
            if game.formula_of(n).is_false() {
                game.p0.incomplete.remove(&n);
                strategy.try_add(n, NodeP1Id::W1);
                game.set_p0_losing(n, strategy);
                return;
            }

            // TODO: use profiles to avoid non-improving moves.
            _ = profiles;

            let Some(pos) = game.p0.moves[n].next() else {
                game.p0.incomplete.remove(&n);
                return;
            };

            let inserted = game.insert_p1(pos);
            game.insert_p0_to_p1_edge(n, inserted.id());
            strategy.try_add(n, inserted.id());

            if let Inserted::New(p1) = inserted {
                add(game.p1.ids[p1]);
            }
        }
        NodeKind::P1(n) => {
            // Handle case where node never had any moves.
            if game.p1.pos[n].moves.is_empty() {
                game.p1.incomplete.remove(&n);
                game.set_p1_losing(n, strategy);
                return;
            }

            // TODO: Make this an input setting?
            // Toggles symmetric and asymmetric algorithm
            const SYMMETRIC: bool = true;

            if SYMMETRIC {
                // Symmetric version: consider next position
                let Some(pos) = game.p1.moves[n].next() else {
                    game.p1.incomplete.remove(&n);
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

                game.p1.incomplete.remove(&n);
            }
        }
    }
}
