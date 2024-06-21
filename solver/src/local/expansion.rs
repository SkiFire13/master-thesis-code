use std::cmp::Ordering;

use indexmap::IndexSet;

use crate::index::{AsIndex, IndexedVec};
use crate::local::game::WinState;
use crate::strategy::{NodeId, PlayProfile, Player};
use crate::symbolic::moves::Assumption;

use super::game::{Game, GameStrategy, Inserted, NodeKind, NodeP1Id};

// Expand the game starting from nodes that are losing for the player controlling them.
// Returns whether any improvement has occurred.
pub fn expand(
    game: &mut Game,
    profiles: &mut IndexedVec<NodeId, PlayProfile>,
    final_strategy: &mut IndexedVec<NodeId, NodeId>,
    strategy: &mut GameStrategy,
    explore_goal: usize,
) -> bool {
    let mut explored = 0;
    let mut improved = false;
    // Explore a minimum amount of nodes and at least until an improvement is found.
    while explored < explore_goal || !improved {
        // Select starting node depending on who's currently winning.
        let start = match profiles[NodeId::INIT].winning(game) {
            Player::P0 => game.p1.incomplete.last().map(|&p1| game.p1.ids[p1]),
            Player::P1 => game.p0.incomplete.last().map(|&p0| game.p0.ids[p0]),
        };

        // If there's no node to expand then return.
        let Some(start) = start else { return !improved };

        // Expand the initial node and save its new successor for later.
        let Some(mut next) = expand_one(start, game, strategy) else { continue };
        let start_next = next.id();

        // Expand one step at a time until an existing node or a loop are found.
        let mut expanded = IndexSet::new();
        let stop = loop {
            let n = match next {
                Inserted::New(n) => n,
                Inserted::Existing(n) => break n,
            };
            expanded.insert(n);
            next = expand_one(n, game, strategy).unwrap();
            final_strategy.push(next.id());
            explored += 1;
        };

        // Incrementally compute the play profiles of the expanded nodes
        update_profiles(stop, &expanded, game, profiles);

        // See if this improves the profile of start. If it does then update the strategy.
        let ord = PlayProfile::compare(game, profiles, start, final_strategy[start], start_next);
        let player = game.player_of(start);
        if let (Ordering::Less, Player::P0) | (Ordering::Greater, Player::P1) = (ord, player) {
            if let NodeKind::P0(p0) = game.resolve(start) {
                let p1 = match game.resolve(start_next) {
                    NodeKind::L0 | NodeKind::W0 | NodeKind::P0(_) => unreachable!(),
                    NodeKind::L1 => NodeP1Id::L1,
                    NodeKind::W1 => NodeP1Id::W1,
                    NodeKind::P1(p1) => p1,
                };
                strategy.update(p0, p1);
            }
            final_strategy[start] = start_next;
            improved = true;
            // TODO: early exit if winner changed?
        }
    }

    false
}

fn expand_one(n: NodeId, game: &mut Game, strategy: &mut GameStrategy) -> Option<Inserted<NodeId>> {
    match game.resolve(n) {
        NodeKind::W0 | NodeKind::L0 | NodeKind::W1 | NodeKind::L1 => unreachable!(),
        NodeKind::P0(p0) => {
            game.p0.moves[p0].simplify(|p| match game.p0.pos.get_index_of(&p) {
                Some(p0) => match game.p0.win[p0] {
                    WinState::Unknown => Assumption::Unknown,
                    WinState::Win0 => Assumption::Winning,
                    WinState::Win1 => Assumption::Losing,
                },
                None => Assumption::Unknown,
            });

            let Some(pos) = game.p0.moves[p0].next() else {
                game.p0.incomplete.swap_remove(&p0);

                // Simplification removed all edges (if there were any)
                if game.p0.succs[p0].is_empty() {
                    game.p0.w1.insert(p0);
                    strategy.try_add(p0, NodeP1Id::W1);
                    return Some(Inserted::Existing(NodeId::W1));
                }

                return None;
            };

            let inserted = game.insert_p1(pos);
            game.insert_p0_to_p1_edge(p0, inserted.id());
            strategy.try_add(p0, inserted.id());

            Some(inserted.map(|p1| game.p1.ids[p1]))
        }
        NodeKind::P1(p1) => {
            // Find move that is not definitely losing for p1.
            let mov = game.p1.moves[p1].by_ref().find(|pos| {
                let Some(p0) = game.p0.pos.get_index_of(pos) else { return true };
                game.p0.win[p0] != WinState::Win0
            });

            let Some(pos) = mov else {
                game.p1.incomplete.swap_remove(&p1);

                // Simplification removed all the edges (if there were any)
                if game.p1.succs[p1].is_empty() {
                    game.p1.w0.insert(p1);
                    return Some(Inserted::Existing(NodeId::W0));
                }

                return None;
            };

            let inserted = game.insert_p0(pos);
            game.insert_p1_to_p0_edge(p1, inserted.id());

            Some(inserted.map(|p0| game.p0.ids[p0]))
        }
    }
}

fn update_profiles(
    stop: NodeId,
    expanded: &IndexSet<NodeId>,
    game: &Game,
    profiles: &mut IndexedVec<NodeId, PlayProfile>,
) {
    let stop_is_expanded = stop.to_usize() >= profiles.len();
    profiles.resize_with(game.nodes.len(), PlayProfile::default);

    let updated_profile = |n, prev_profile: &PlayProfile| {
        let mut profile = prev_profile.clone();

        // Update relevant_before
        let n_rel = game.relevance_of(n);
        if n_rel > game.relevance_of(prev_profile.most_relevant) {
            let pos = profile.relevant_before.partition_point(|&m| game.relevance_of(m) > n_rel);
            profile.relevant_before.insert(pos, n);
        }

        // Update count_before
        profile.count_before += 1;

        profile
    };

    if stop_is_expanded {
        // The expansion created a cycle within itself; find it and the most relevant node of the cycle.
        let cycle_start = expanded.get_index_of(&stop).unwrap();
        let most_relevant =
            expanded[cycle_start..].iter().copied().max_by_key(|&n| game.relevance_of(n)).unwrap();
        let most_relevant_index = expanded.get_index_of(&most_relevant).unwrap();

        // Set the profile of the most relevant node of the cycle
        profiles[most_relevant].most_relevant = most_relevant;

        // Set the profile of the expanded nodes before the cycle.
        let mut next = most_relevant;
        for &n in expanded[..most_relevant_index].iter().rev() {
            profiles[n] = updated_profile(n, &profiles[next]);
            next = n;
        }

        // Set profiles of the expanded nodes in the cycle.
        let mut next = stop;
        for &n in expanded[most_relevant_index + 1..].iter().rev() {
            // We're in the loop so there's no node more relevant than `most_relevant`
            profiles[n].most_relevant = most_relevant;
            profiles[n].count_before = profiles[next].count_before + 1;
            next = n;
        }
    } else {
        // The expansion reached the existing graph, update linearly the play profiles
        let mut next = stop;
        for &n in expanded.iter().rev() {
            profiles[n] = updated_profile(n, &profiles[next]);
            next = n;
        }
    }
}
