use std::collections::VecDeque;

use crate::index::IndexedVec;

use super::{
    GetRelevance, NodeId, NodeMap, ParityGraph, PlayProfile, Player, Relevance, Set, Strategy,
};

struct Graph<'a, S: Strategy> {
    game: &'a S::Graph,
    strategy: &'a S,
}

impl<'a, S: Strategy> Graph<'a, S> {
    fn predecessors_of(&self, n: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.strategy.predecessors_of(n, self.game)
    }

    fn successors_of(&self, n: NodeId) -> impl Iterator<Item = NodeId> + 'a {
        self.strategy.successors_of(n, self.game)
    }

    fn node_count(&self) -> usize {
        self.game.node_count()
    }

    fn relevance_of(&self, n: NodeId) -> Relevance {
        self.game.relevance_of(n)
    }

    fn nodes_sorted_by_reward(&self) -> impl Iterator<Item = NodeId> + 'a {
        self.game.nodes_sorted_by_reward()
    }
}

pub fn valuation<S: Strategy>(
    game: &S::Graph,
    strategy: &S,
) -> (IndexedVec<NodeId, PlayProfile>, IndexedVec<NodeId, NodeId>) {
    // Build graph with p0 moves restricted to the given strategy.
    let graph = &Graph { game, strategy };

    let mut evaluated = Set::default();
    let mut profiles = IndexedVec::from(vec![PlayProfile::default(); graph.node_count()]);
    let mut final_strategy = IndexedVec::from(vec![NodeId(usize::MAX); graph.node_count()]);

    // Iterate by reward order, i.e. first nodes that are more in favour of player 1.
    // At each iteration we will try to fix all the loops that go through w, if w is not
    // already part of one.
    for w in graph.nodes_sorted_by_reward() {
        // Ignore already evaluated nodes
        if evaluated.contains(&w) {
            continue;
        }

        // Consider only predecessors and successors that weren't "removed".
        let preds_of = |n| graph.predecessors_of(n).filter(|v| !evaluated.contains(v));
        let succs_of = |n| graph.successors_of(n).filter(|v| !evaluated.contains(v));

        // Find all nodes v <= w that can reach w
        let rel_of = |v| graph.relevance_of(v);
        let w_rel = rel_of(w);
        let reach_set = reach(w, |u| preds_of(u).filter(|&v| rel_of(v) <= w_rel));
        // We want to check if w can reach itself with a non-self path,
        // so check if any of its successors can reach it.
        if !succs_of(w).any(|v| reach_set.contains(&v)) {
            continue;
        }

        // Find all nodes that can reach w, we will make them go through w.
        let k_set = reach(w, preds_of);

        // Subevaluation: force all cycles that contain w to happen,
        // with the best path possible.
        subevaluation(graph, w, &k_set, &mut profiles, &mut final_strategy);

        // Equivalent to removing edges from K to V \ K,
        // as it will make sure they will never get explored again.
        evaluated.extend(k_set);
    }

    (profiles, final_strategy)
}

/// A graph restricted to only some nodes (k) and with some edges removed.
struct RestrictedGraph<'a, S: Strategy> {
    /// The base graph
    base: &'a Graph<'a, S>,
    /// List of nodes that are in the restricted graph (for fast iteration)
    k_nodes: &'a [NodeId],
    /// Set of nodes that are in the restricted graph (for filtering/checking)
    k_set: &'a Set<NodeId>,
    /// Edges removed from the graph
    removed_edges: Set<(NodeId, NodeId)>,
    /// Number of outgoing edges removed from each node, used to quickly compute number of successors.
    removed_successors_count: NodeMap<usize>,
}

impl<'a, S: Strategy> RestrictedGraph<'a, S> {
    fn predecessors_of(&self, v: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.base
            .predecessors_of(v)
            .filter(|&u| self.k_set.contains(&u))
            .filter(move |&u| !self.removed_edges.contains(&(u, v)))
    }

    fn successors_of(&self, v: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.base
            .successors_of(v)
            .filter(|&u| self.k_set.contains(&u))
            .filter(move |&u| !self.removed_edges.contains(&(v, u)))
    }

    fn successors_count_of(&self, v: NodeId) -> usize {
        // Take all successors and consider only those in the original K.
        // Then remove from the count those edges that were removed.
        self.base.successors_of(v).filter(|u| self.k_set.contains(u)).count()
            - self.removed_successors_count.get(&v).unwrap_or(&0)
    }

    fn all_successors_of(&self, v: NodeId) -> impl Iterator<Item = NodeId> + 'a {
        self.base.successors_of(v)
    }

    fn remove_edge(&mut self, v: NodeId, u: NodeId) {
        if self.k_set.contains(&u) && self.removed_edges.insert((v, u)) {
            *self.removed_successors_count.entry(v).or_insert(0) += 1;
        }
    }

    fn relevance_of(&self, v: NodeId) -> Relevance {
        self.base.relevance_of(v)
    }
}

fn subevaluation(
    graph: &Graph<impl Strategy>,
    w: NodeId,
    k_set: &Set<NodeId>,
    profiles: &mut IndexedVec<NodeId, PlayProfile>,
    final_strategy: &mut IndexedVec<NodeId, NodeId>,
) {
    let mut k_nodes = k_set.iter().copied().collect::<Vec<_>>();

    // Sort K by relevance, for the loop later on.
    k_nodes.sort_by_key(|&v| graph.relevance_of(v));

    let mut graph = RestrictedGraph {
        base: graph,
        k_nodes: &k_nodes,
        k_set,
        removed_edges: Set::default(),
        removed_successors_count: NodeMap::default(),
    };

    let w_relevance = graph.relevance_of(w);

    // All these nodes will be part of cycles that contain w as most relevant node.
    for &v in &*graph.k_nodes {
        profiles[v].most_relevant = w;
    }

    let mut reach = Reach { stack: Vec::new(), set: Set::default() };

    // Iterate over K with descending relevance order, considering only those
    // nodes that have higher relevance than w.
    for &u in graph.k_nodes.iter().rev() {
        if graph.relevance_of(u) <= w_relevance {
            break;
        }

        // Optimization:
        // If u has no predecessor there will be no path to prevent or force,
        // so the only thing left to do is set the `relevant_before` of `u`.
        // This can avoid an expensive `reach` call for `prevent_paths`.
        if graph.predecessors_of(u).next().is_none() {
            profiles[u].relevant_before.push(u);
            continue;
        }

        match graph.relevance_of(u).player() {
            Player::P0 => prevent_paths(&mut graph, w, u, profiles, &mut reach),
            Player::P1 => force_paths(&mut graph, w, u, profiles, &mut reach),
        }
    }

    // Depending on the player favoured by w maximize or minimize the distances.
    match graph.relevance_of(w).player() {
        Player::P0 => set_maximal_distances(&mut graph, w, profiles, final_strategy),
        Player::P1 => set_minimal_distances(&mut graph, w, profiles, final_strategy),
    }
}

/// Prevent any path that can go through u from doing so.
fn prevent_paths(
    graph: &mut RestrictedGraph<impl Strategy>,
    w: NodeId,
    u: NodeId,
    profiles: &mut IndexedVec<NodeId, PlayProfile>,
    reach: &mut Reach,
) {
    // Find nodes that can reach w without going through u.
    let u_set = reach.reach(w, |n| graph.predecessors_of(n).filter(|&v| v != u));

    // Update profiles of nodes whose path must go through u.
    for &v in graph.k_nodes.iter().filter(|&v| !u_set.contains(v)) {
        profiles[v].relevant_before.push(u);
    }

    // Remove edges that would make paths go through u when it's possible
    // to avoid it, that is edges from u_nodes U {u} to V \ U_nodes.
    for &v in u_set.iter().chain([&u]) {
        for next in graph.all_successors_of(v).filter(|n| !u_set.contains(n)) {
            graph.remove_edge(v, next);
        }
    }
}

/// Make any path that can go through u do so.
fn force_paths(
    graph: &mut RestrictedGraph<impl Strategy>,
    w: NodeId,
    u: NodeId,
    profiles: &mut IndexedVec<NodeId, PlayProfile>,
    reach: &mut Reach,
) {
    // Find nodes that can reach u without going through w.
    let u_set = reach.reach(u, |n| graph.predecessors_of(n).filter(|&v| v != w));

    // Update profiles of nodes whose path can go through u.
    for &v in u_set {
        profiles[v].relevant_before.push(u);
    }

    // Remove edges that would make paths not go through u when it's possible
    // to do so, that is edges from u_nodes \ {u} to V \ u_nodes
    for &v in u_set.iter().filter(|&&v| v != u) {
        for next in graph.all_successors_of(v).filter(|n| !u_set.contains(n)) {
            graph.remove_edge(v, next);
        }
    }
}

struct Reach {
    stack: Vec<NodeId>,
    set: Set<NodeId>,
}

impl Reach {
    fn reach<F, I>(&mut self, start: NodeId, mut explore: F) -> &Set<NodeId>
    where
        F: FnMut(NodeId) -> I,
        I: Iterator<Item = NodeId>,
    {
        self.stack.push(start);
        self.set.clear();
        self.set.insert(start);

        // DFS according to explore
        while let Some(node) = self.stack.pop() {
            explore(node).for_each(|next| {
                if self.set.insert(next) {
                    self.stack.push(next);
                }
            });
        }

        &self.set
    }
}

fn reach<F, I>(start: NodeId, explore: F) -> Set<NodeId>
where
    F: FnMut(NodeId) -> I,
    I: Iterator<Item = NodeId>,
{
    let mut out = Reach { stack: Vec::new(), set: Set::default() };
    out.reach(start, explore);
    out.set
}

fn set_maximal_distances(
    graph: &mut RestrictedGraph<impl Strategy>,
    w: NodeId,
    profiles: &mut IndexedVec<NodeId, PlayProfile>,
    final_strategy: &mut IndexedVec<NodeId, NodeId>,
) {
    let mut remaining_successors =
        graph.k_nodes.iter().map(|&v| (v, graph.successors_count_of(v))).collect::<NodeMap<_>>();
    let mut queue = VecDeque::from([(w, graph.successors_of(w).next().unwrap(), 0)]);

    while let Some((v, succ, d)) = queue.pop_front() {
        profiles[v].count_before = d;
        final_strategy[v] = succ;

        for u in graph.predecessors_of(v).filter(|&u| u != w) {
            // Decrease number of remaining successors to visit
            let remaining = remaining_successors.get_mut(&u).unwrap();
            *remaining -= 1;

            // If last was visited then add node to the queue with one more edge.
            if *remaining == 0 {
                queue.push_back((u, v, d + 1));
            }
        }
    }
}

fn set_minimal_distances(
    graph: &mut RestrictedGraph<impl Strategy>,
    w: NodeId,
    profiles: &mut IndexedVec<NodeId, PlayProfile>,
    final_strategy: &mut IndexedVec<NodeId, NodeId>,
) {
    let mut seen = Set::default();
    let mut queue = VecDeque::from([(w, graph.successors_of(w).next().unwrap(), 0)]);

    // Backward BFS
    while let Some((v, succ, d)) = queue.pop_front() {
        if seen.insert(v) {
            profiles[v].count_before = d;
            final_strategy[v] = succ;
            queue.extend(graph.predecessors_of(v).map(|u| (u, v, d + 1)))
        }
    }
}
