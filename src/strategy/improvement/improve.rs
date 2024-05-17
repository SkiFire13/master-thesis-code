use crate::index::IndexVec;
use crate::strategy::game::{NodeId, NodeP0Id, NodeP1Id};

use super::profile::{GetRelevance, PlayProfile};

pub trait ImproveGraph: GetRelevance {
    fn p0_successors(&self, n: NodeP0Id) -> impl Iterator<Item = NodeP1Id>;
    fn p1_to_node(&self, n: NodeP1Id) -> NodeId;
}

pub fn improve(
    graph: &impl ImproveGraph,
    strategy: &mut IndexVec<NodeP0Id, NodeP1Id>,
    profiles: &IndexVec<NodeId, PlayProfile>,
) -> bool {
    let mut improved = false;

    // For each p0 node try improving it
    for (n0, n1) in strategy.enumerate_mut() {
        // For each successor check if its play profile is better
        for m1 in graph.p0_successors(n0) {
            let n1id = graph.p1_to_node(*n1);
            let m1id = graph.p1_to_node(m1);
            if profiles[n1id].cmp(&profiles[m1id], graph).is_lt() {
                // If it's better update the strategy
                *n1 = m1;
                improved = true;
            }
        }
    }

    improved
}
