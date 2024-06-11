use crate::index::IndexedVec;
use crate::strategy::game::Player;

use super::game::{Game, GameStrategy, NodeId, NodeP1Id, WinState};
use super::improvement::PlayProfile;
use super::Set;

pub fn update_winning_sets(
    game: &mut Game,
    profiles: &IndexedVec<NodeId, PlayProfile>,
    final_strategy: &IndexedVec<NodeId, NodeId>,
    strategy: &mut GameStrategy,
) {
    // Gather the nodes that have not been fully explored yet.
    let p0_incomplete = game.p0.incomplete.iter().map(|&n| game.p0.ids[n]);
    let p1_incomplete = game.p1.incomplete.iter().map(|&n| game.p1.ids[n]);
    let incomplete = p0_incomplete.chain(p1_incomplete);

    // Find nodes that are transitively reach unexplored ones, assuming the optimal strategy for the opponent.
    let escaping = find_escape_set(incomplete, |n| game.predecessors_of(n), final_strategy);

    // TODO: make these loops generic?
    // TODO: Maybe avoid iterating over all nodes?

    for (p0, &n0) in game.p0.ids.enumerate() {
        let is_losing = profiles[n0].winning(game) == Player::P1 && !escaping.contains(&n0);
        let is_already_losing = game.p0.win[p0] == WinState::Win1;

        if !is_losing || is_already_losing {
            continue;
        }

        // Mark nodes as losing.
        game.p0.win[p0] = WinState::Win1;
        game.p0.w1.insert(p0);

        // Fixup P0 strategy
        strategy.update(p0, NodeP1Id::W1);

        // Optimization: remove successors of predecessors
        for p1 in std::mem::take(&mut game.p0.preds[p0]) {
            debug_assert_eq!(game.p1.win[p1], WinState::Unknown);

            // Mark predecessors as winning.
            game.p1.win[p1] = WinState::Win1;
            game.p1.w1.insert(p1);

            // Optimization: remove successors of predecessors
            for p0 in std::mem::take(&mut game.p1.succs[p1]) {
                game.p0.preds[p0].remove(&p1);
            }
        }

        // Optimization: remove successors
        for p1 in std::mem::take(&mut game.p0.succs[p0]) {
            game.p1.preds[p1].remove(&p0);
        }
    }

    for (p1, &n1) in game.p1.ids.enumerate() {
        let is_losing = profiles[n1].winning(game) == Player::P0 && !escaping.contains(&n1);
        let is_already_losing = game.p1.win[p1] == WinState::Win0;

        if !is_losing || is_already_losing {
            continue;
        }

        // Mark nodes as losing.
        game.p1.win[p1] = WinState::Win0;
        game.p1.w0.insert(p1);

        // Optimization: remove successors of predecessors
        for p0 in std::mem::take(&mut game.p1.preds[p1]) {
            debug_assert_eq!(game.p0.win[p0], WinState::Unknown);

            // Mark predecessors as winning.
            game.p0.win[p0] = WinState::Win0;
            game.p0.w0.insert(p0);

            // Optimization: remove successors of predecessors
            for p1 in std::mem::take(&mut game.p0.succs[p0]) {
                game.p1.preds[p1].remove(&p0);
            }

            // Fixup P0 strategy
            strategy.update(p0, NodeP1Id::L1);
        }

        // Optimization: remove successors
        for p0 in std::mem::take(&mut game.p1.succs[p1]) {
            game.p0.preds[p0].remove(&p1);
        }
    }
}

// TODO: Test this
// For each player find the nodes that can reach escaping nodes
// assuming the opponent player plays the given strategy.
// This assumes a bipartite graph.
fn find_escape_set<I: Iterator<Item = NodeId>>(
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
