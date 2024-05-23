use crate::index::IndexedVec;
use crate::strategy::game::Player;

use super::game::{Game, NodeId};
use super::improvement::PlayProfile;
use super::Set;

pub fn update_w01(
    game: &mut Game,
    profiles: &IndexedVec<NodeId, PlayProfile>,
    strategy: &IndexedVec<NodeId, NodeId>,
) {
    // Find nodes that are transitively escaping, assuming the optimal strategy for the opponent.
    let escaping = find_escaping(
        game.escaping.iter().copied(),
        |n| game.predecessors_of(n),
        strategy,
    );

    // TODO: Maybe avoid iterating over all nodes?
    for (p0, &n0) in game.p0.node_ids.enumerate() {
        let losing = game.relevance_of(profiles[n0].most_relevant).player() == Player::P1;
        if losing && !escaping.contains(&n0) {
            game.p0.w1.push(p0);

            for &p1 in &game.p0.preds[p0] {
                game.p1.w1.push(p1);
                // TODO: Something to update?
            }

            // TODO: Update predecessors of successors of p0
            game.p0.succs[p0].clear();
        }
    }

    for (p1, &n1) in game.p1.node_ids.enumerate() {
        let losing = game.relevance_of(profiles[n1].most_relevant).player() == Player::P0;
        if losing && !escaping.contains(&n1) {
            game.p1.w0.push(p1);

            for &p0 in &game.p1.preds[p1] {
                game.p0.w0.push(p0);
                // TODO: Something to update?
            }

            // TODO: Update predecessors of successors of p0?
            game.p1.succs[p1].clear();
        }
    }
}

// TODO: Test this
fn find_escaping<I: Iterator<Item = NodeId>>(
    escaping: impl Iterator<Item = NodeId>,
    predecessors_of: impl Fn(NodeId) -> I,
    strategy: &IndexedVec<NodeId, NodeId>,
) -> Set<NodeId> {
    let mut inverse_strategy = IndexedVec::from(vec![Vec::new(); strategy.len()]);
    for (n, &m) in strategy.enumerate() {
        inverse_strategy[m].push(n);
    }

    // Find nodes that are transitively escaping, assuming the optimal strategy for the opponent.
    let mut queue = escaping.collect::<Vec<_>>();
    let mut escaping = queue.iter().copied().collect::<Set<_>>();

    while let Some(n) = queue.pop() {
        queue.extend(
            // Only consider edges to n according to the opposing player strategy.
            inverse_strategy[n]
                .iter()
                // Then consider all predecessors, since those are controlled by n's player
                .flat_map(|&n| predecessors_of(n))
                // Add the nodes as escaping and filter those already seen.
                .filter(|&n| escaping.insert(n)),
        );
    }

    escaping
}
