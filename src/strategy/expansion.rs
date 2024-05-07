use std::collections::HashSet;

use crate::strategy::game::{NodeId, NodeP0Id, Player};

use super::game::{Game, NodeData};

pub fn expand(game: &mut Game) {
    let mut a = e1(game);
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

fn e1(game: &mut Game) -> Vec<NodeData> {
    let init_node = NodeId::INIT;
    let relevant_node = game.profiles[init_node.0].0;

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

fn e2(game: &mut Game, w: NodeData, mut add: impl FnMut(NodeData)) {
    match w {
        // TODO: will these ever be hit?
        NodeData::W0 => add(NodeData::L1),
        NodeData::L0 => add(NodeData::W1),
        NodeData::W1 => add(NodeData::L0),
        NodeData::L1 => add(NodeData::W0),
        NodeData::P0(n) => {
            // TODO: apply decisions and stuff
            todo!();
        }
        NodeData::P1(n) => {
            for &bi in &game.nodes_p1[n.0] {
                if game.nodes_p0.get(&bi).is_none() {
                    let (idx, _) = game.nodes_p0.insert_full(bi);
                    add(NodeData::P0(NodeP0Id(idx)))
                    // TODO: Add forward and backward edges

                    // Only in synchronous version:
                    // break;
                }
            }
        }
    }

    todo!()
}
