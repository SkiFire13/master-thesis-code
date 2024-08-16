use crate::index::IndexedVec;
use crate::strategy::{NodeId, PlayProfile, Set};

use super::game::{Game, GameStrategy, NodeKind, WinState};

pub fn update_winning_sets(
    game: &mut Game,
    profiles: &IndexedVec<NodeId, PlayProfile>,
    final_strategy: &mut IndexedVec<NodeId, NodeId>,
    strategy: &mut GameStrategy,
) {
    // Actually set the losing nodes as losing. This will also mark the predecessors as winning.
    for n in definitely_losing_set(game, profiles, final_strategy) {
        match game.resolve(n) {
            NodeKind::P0(p0) if game.p0.win[p0] == WinState::Unknown => {
                game.set_p0_losing(p0, strategy, final_strategy)
            }
            NodeKind::P1(p1) if game.p1.win[p1] == WinState::Unknown => {
                game.set_p1_losing(p1, strategy, final_strategy)
            }
            _ => {}
        }
    }
}

fn definitely_losing_set(
    game: &Game,
    profiles: &IndexedVec<NodeId, PlayProfile>,
    final_strategy: &IndexedVec<NodeId, NodeId>,
) -> Set<NodeId> {
    // Precompute the inverse strategy, we will use it later to iterate over the strategy predecessors.
    let mut inverse_strategy = IndexedVec::from(vec![Vec::new(); final_strategy.len()]);
    for (n, &m) in final_strategy.enumerate() {
        inverse_strategy[m].push(n);
    }

    // Compute the sets of losing nodes on the subgame.
    let mut losing = profiles
        .enumerate()
        .filter(|&(n, p)| game.player_of(n) != p.winning(game))
        .map(|(n, _)| n)
        .collect::<Set<_>>();

    // Incomplete nodes are not losing because they might reach new nodes and become winning.
    // Remove them from the definitely losing set and them to a queue to recursively mark the
    // predecessors too.
    let p0_incomplete = game.p0.incomplete.iter().map(|&n| game.p0.ids[n]);
    let p1_incomplete = game.p1.incomplete.iter().map(|&n| game.p1.ids[n]);
    let mut queue =
        p0_incomplete.chain(p1_incomplete).filter(|n| losing.swap_remove(n)).collect::<Vec<_>>();

    // For each node that is losing in the subgame, but can escape it, consider its
    // predecessors according to the opposing player's optimal strategy. They will not be
    // definitely winning. In turn all the predecessors of that node won't be definitely losing.
    while let Some(n) = queue.pop() {
        for &p in &inverse_strategy[n] {
            queue.extend(game.predecessors_of(p).filter(|pp| losing.swap_remove(pp)));
        }
    }

    losing
}
