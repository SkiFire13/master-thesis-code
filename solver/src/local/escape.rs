use crate::index::IndexedVec;
use crate::strategy::{NodeId, PlayProfile, Player, Set};

use super::game::{Game, GameStrategy, WinState};

pub fn update_winning_sets(
    game: &mut Game,
    profiles: &IndexedVec<NodeId, PlayProfile>,
    final_strategy: &mut IndexedVec<NodeId, NodeId>,
    strategy: &mut GameStrategy,
) {
    // Gather the nodes that have not been fully explored yet.
    let p0_incomplete = game.p0.incomplete.iter().map(|&n| game.p0.ids[n]);
    let p1_incomplete = game.p1.incomplete.iter().map(|&n| game.p1.ids[n]);
    let incomplete = p0_incomplete.chain(p1_incomplete);

    // Find nodes that are transitively reach unexplored ones, assuming the optimal strategy for the opponent.
    let escape_set = find_escape_set(incomplete, |n| game.predecessors_of(n), final_strategy);

    for p0 in game.p0.ids.indexes() {
        let n0 = game.p0.ids[p0];

        let is_losing = profiles[n0].winning(game) == Player::P1 && !escape_set.contains(&n0);
        let is_already_losing = game.p0.win[p0] == WinState::Win1;

        if !is_losing || is_already_losing {
            continue;
        }

        game.set_p0_losing(p0, strategy, final_strategy);
    }

    for p1 in game.p1.ids.indexes() {
        let n1 = game.p1.ids[p1];

        let is_losing = profiles[n1].winning(game) == Player::P0 && !escape_set.contains(&n1);
        let is_already_losing = game.p1.win[p1] == WinState::Win0;

        if !is_losing || is_already_losing {
            continue;
        }

        game.set_p1_losing(p1, strategy, final_strategy);
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
