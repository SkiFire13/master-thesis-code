use std::cmp::{Ordering, Reverse};

// TODO: Use node/vertex consistently

// Bitset or something similar?
pub type Set<T> = std::collections::BTreeSet<T>;

pub struct Graph {}

impl Graph {
    fn successors_of(&self, n: NodeId) -> impl Iterator<Item = NodeId> {
        todo!();
        [].into_iter()
    }

    fn predecessors_of(&self, n: NodeId) -> impl Iterator<Item = NodeId> {
        todo!();
        [].into_iter()
    }

    fn node_count(&self) -> usize {
        todo!()
    }

    fn relevance_of(&self, n: NodeId) -> usize {
        todo!()
    }

    fn nodes_sorted_by_reward(&self) -> impl Iterator<Item = NodeId> {
        todo!();
        [].into_iter()
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(usize);

#[derive(Clone, Default)]
pub struct PlayProfile(
    /// Most relevant node of the cycle.
    NodeId,
    /// Nodes more relevant visited before the cycle, sorted by most relevant first.
    Vec<NodeId>,
    /// Number of nodes visited before the most relevant of the cycle.
    usize,
);

impl PlayProfile {
    pub fn cmp(&self, other: &PlayProfile, graph: &Graph) -> Ordering {
        // Compare the most relevant vertex of the cycle
        if self.0 != other.0 {
            let self_rel = graph.relevance_of(self.0);
            let other_rel = graph.relevance_of(other.0);

            return match (self_rel % 2, other_rel % 2) {
                // Both in favour of player 0, order is normal
                (0, 0) => Ord::cmp(&(self_rel, self.0), &(other_rel, other.0)),
                // Only first in favour of player 0, that's the greatest
                (0, _) => Ordering::Greater,
                // Only second in favour of player 0, that's the greatest
                (_, 0) => Ordering::Less,
                // Both in favour of player 1, order is reversed
                (_, _) => Ord::cmp(&(self_rel, self.0), &(other_rel, other.0)).reverse(),
            };
        }

        // Compare the more relevant vertexes visited before the most relevant one.
        if let Some((&u, &v)) = std::iter::zip(&self.1, &other.1).find(|(u, v)| u != v) {
            let u_relevance = graph.relevance_of(u);
            let v_relevance = graph.relevance_of(v);

            return match std::cmp::max(u_relevance, v_relevance) % 2 {
                // The biggest is in favour of player 0, order is normal.
                0 => Ord::cmp(&(u_relevance, u), &(v_relevance, v)),
                // The biggest is in favour of player 1, order is reversed.
                _ => Ord::cmp(&(u_relevance, u), &(v_relevance, v)).reverse(),
            };
        }

        // Compare the number of nodes visited before most relevant vertex of the loop
        match graph.relevance_of(self.0) % 2 {
            0 => Ord::cmp(&self.2, &other.2).reverse(),
            _ => Ord::cmp(&self.2, &other.2),
        }
    }
}

pub fn valuation(graph: &Graph) -> Vec<PlayProfile> {
    // TODO: Bitset or something similar?
    let mut evaluated: Set<NodeId> = Set::new();
    let mut profiles = vec![PlayProfile::default(); graph.node_count()];

    for w in graph.nodes_sorted_by_reward() {
        // Ignore already evaluated nodes
        if evaluated.contains(&w) {
            continue;
        }

        let w_relevance = graph.relevance_of(w);

        let predecessors_of = |n| graph.predecessors_of(n).filter(|v| !evaluated.contains(v));

        // Find all nodes v <= w that can reach w
        let mut reach_stack = predecessors_of(w).collect::<Vec<_>>();
        let mut reach_set = Set::new();
        while let Some(v) = reach_stack.pop() {
            if graph.relevance_of(v) <= w_relevance && reach_set.insert(v) {
                reach_stack.extend(predecessors_of(v));
            }
        }

        // If w cannot reach itself it cannot create a loop, ignore it.
        if !reach_set.contains(&w) {
            continue;
        }

        // Find all nodes that can reach w.
        let mut reach_stack = predecessors_of(w).collect::<Vec<_>>();
        let (mut k_nodes, mut k_set) = (Vec::new(), Set::new());
        while let Some(v) = reach_stack.pop() {
            if k_set.insert(v) {
                k_nodes.push(v);
                reach_stack.extend(predecessors_of(v));
            }
        }

        // Subevaluation: force all cycles that contain w to happen,
        // with the best path possible.
        subevaluation(graph, w, &mut k_nodes, &mut profiles, &evaluated);

        // Equivalent to removing edges from K to V \ K,
        // as it will make sure they will never get explored again.
        for &v in &k_nodes {
            evaluated.insert(v);
        }
    }

    profiles
}

fn subevaluation(
    graph: &Graph,
    w: NodeId,
    k_nodes: &mut [NodeId],
    profiles: &mut [PlayProfile],
    evaluated: &Set<NodeId>,
) {
    // TODO: don't ignore this
    let mut removed_edges = Set::new();
    let w_relevance = graph.relevance_of(w);

    // Sort K by relevance
    k_nodes.sort_by_key(|&v| graph.relevance_of(v));

    // All these nodes will be part of cycles that contain w as most relevant node.
    for &v in &*k_nodes {
        profiles[v.0].0 = w;
    }

    // Iterate over K with descending relevance order for those nodes that have
    // higher relevance than w
    for &u in k_nodes.iter().rev() {
        if graph.relevance_of(u) <= w_relevance {
            break;
        }

        if graph.relevance_of(u) % 2 == 0 {
            // The node is in favour of player 0, avoid ever going through it if possible.
            prevent_paths(
                graph,
                w,
                u,
                k_nodes,
                profiles,
                evaluated,
                &mut removed_edges,
            );
        } else {
            // The node is in favour of player 1, try to go through it if possible.
            force_paths(
                graph,
                w,
                u,
                k_nodes,
                profiles,
                evaluated,
                &mut removed_edges,
            )
        }
    }

    // Extra: sort the nodes in the profile by their relevance, as that will help
    // when comparing profiles.
    for v in &*k_nodes {
        profiles[v.0]
            .1
            .sort_by_key(|&n| Reverse(graph.relevance_of(n)));
    }

    if graph.relevance_of(w) % 2 == 0 {
        // TODO: maximal_distances
    } else {
        // TODO: minimal_distances
    }
}

/// Prevent any path that can go through u from doing so.
fn prevent_paths(
    graph: &Graph,
    w: NodeId,
    u: NodeId,
    k_nodes: &[NodeId],
    profiles: &mut [PlayProfile],
    evaluated: &Set<NodeId>,
    removed_edges: &mut Set<(NodeId, NodeId)>,
) {
    let predecessors_of = |n| {
        let removed_edges = &removed_edges;
        graph
            .predecessors_of(n)
            .filter(|&v| !evaluated.contains(&v))
            .filter(move |&v| !removed_edges.contains(&(v, n)))
    };

    // Find nodes reachable from w in the graph excluding u.
    let mut reach_stack = predecessors_of(w).collect::<Vec<_>>();
    let (mut u_nodes, mut u_set) = (Vec::new(), Set::new());
    while let Some(v) = reach_stack.pop() {
        if v != u && u_set.insert(v) {
            u_nodes.push(v);
            reach_stack.extend(predecessors_of(v));
        }
    }

    // Update profiles of those path that must go through u
    for &v in k_nodes.iter().filter(|v| !u_set.contains(v)) {
        profiles[v.0].1.push(u);
    }

    // Remove edges from u_nodes U {u} to V \ U_nodes
    for &v in u_nodes.iter().chain([&u]) {
        for next in graph.successors_of(v).filter(|next| !u_set.contains(next)) {
            removed_edges.insert((v, next));
        }
    }
}

/// Make any path that can go through u do so.
fn force_paths(
    graph: &Graph,
    w: NodeId,
    u: NodeId,
    k_nodes: &[NodeId],
    profiles: &mut [PlayProfile],
    evaluated: &Set<NodeId>,
    removed_edges: &mut Set<(NodeId, NodeId)>,
) {
    let predecessors_of = |n| {
        let removed_edges = &removed_edges;
        graph
            .predecessors_of(n)
            .filter(|&v| !evaluated.contains(&v))
            .filter(move |&v| !removed_edges.contains(&(v, n)))
    };

    // Find nodes reachable from w in the graph excluding u.
    let mut reach_stack = predecessors_of(u).collect::<Vec<_>>();
    let (mut u_nodes, mut u_set) = (Vec::new(), Set::new());
    while let Some(v) = reach_stack.pop() {
        if v != w && u_set.insert(v) {
            u_nodes.push(v);
            reach_stack.extend(predecessors_of(v));
        }
    }

    // Update profiles of those path that can go through u
    for &v in k_nodes.iter().filter(|v| u_set.contains(v)) {
        profiles[v.0].1.push(u);
    }

    // Remove edges from u_nodes \ {u} to V \ u_nodes
    for &v in u_nodes.iter().filter(|&&v| v != u) {
        for next in graph.successors_of(v).filter(|next| !u_set.contains(next)) {
            removed_edges.insert((v, next));
        }
    }
}
