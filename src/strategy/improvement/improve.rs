use crate::index::IndexVec;
use crate::strategy::game::{NodeId, NodeP0Id, NodeP1Id};

use super::profile::{GetRelevance, PlayProfile};
use super::valuation::{Strategy, ValuationGraph};

pub trait ImproveGraph: GetRelevance {
    fn p0_successors(&self, n: NodeP0Id) -> impl Iterator<Item = NodeP1Id>;
    fn p1_to_node(&self, n: NodeP1Id) -> NodeId;
}

pub trait StrategyMut: Strategy {
    fn update_each(&mut self, graph: &Self::Graph, f: impl FnMut(NodeId, NodeId) -> NodeId);
}

pub fn improve<S: StrategyMut>(
    graph: &S::Graph,
    strategy: &mut S,
    profiles: &IndexVec<NodeId, PlayProfile>,
) -> bool {
    let mut improved = false;

    // For each node in the strategy try to improve it.
    strategy.update_each(graph, |n0, mut n1| {
        // For each successor check if its play profile is better
        for n2 in graph.successors_of(n0) {
            if profiles[n1].cmp(&profiles[n2], graph).is_lt() {
                // If it's better update the strategy
                n1 = n2;
                improved = true;
            }
        }
        n1
    });

    improved
}
