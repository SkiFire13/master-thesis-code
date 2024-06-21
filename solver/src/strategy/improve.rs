use crate::index::IndexedVec;

use super::{NodeId, ParityGraph, PlayProfile, StrategyMut};

pub fn improve<S: StrategyMut>(
    graph: &S::Graph,
    strategy: &mut S,
    profiles: &IndexedVec<NodeId, PlayProfile>,
) -> bool {
    let mut improved = false;

    // For each node in the strategy try to improve it.
    strategy.update_each(graph, |n0, mut n1| {
        // For each successor check if its play profile is better
        for n2 in graph.successors_of(n0) {
            // Compare the profiles of n1 and n2
            if PlayProfile::compare(graph, profiles, n0, n1, n2).is_lt() {
                // If it's better update the strategy
                n1 = n2;
                improved = true;
            }
        }
        n1
    });

    improved
}
