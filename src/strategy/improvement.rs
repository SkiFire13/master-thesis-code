use std::cmp::{Ordering, Reverse};
use std::collections::VecDeque;

// TODO: Use node/vertex consistently

// Bitset or something similar?
pub type Set<T> = std::collections::BTreeSet<T>;
pub type NodeMap<T> = std::collections::HashMap<NodeId, T>;

pub struct Graph {}

impl Graph {
    fn successors_of(&self, n: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        todo!();
        [].into_iter()
    }

    fn predecessors_of(&self, n: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        todo!();
        [].into_iter()
    }

    fn node_count(&self) -> usize {
        todo!()
    }

    fn relevance_of(&self, n: NodeId) -> Relevance {
        todo!()
    }

    fn nodes_sorted_by_reward(&self) -> impl Iterator<Item = NodeId> + '_ {
        todo!();
        [].into_iter()
    }

    fn successors_count_of(&self, n: NodeId) -> usize {
        todo!()
    }
}

pub enum Player {
    P0,
    P1,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Relevance(usize, NodeId);

impl Relevance {
    pub fn player(self) -> Player {
        match self.0 % 2 {
            0 => Player::P0,
            _ => Player::P1,
        }
    }

    pub fn reward(self) -> Reward {
        match self.player() {
            Player::P0 => Reward::P0(self),
            Player::P1 => Reward::P1(Reverse(self)),
        }
    }
}

// Note: order is important here. Reward in favour of P1 are considered less
// than rewards in favour of P0. Also, relevance for P1 rewards are considered
// reversed (bigger relevance is worse for P0, and thus less).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reward {
    P1(Reverse<Relevance>),
    P0(Relevance),
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
            let this_rew = graph.relevance_of(self.0).reward();
            let that_rew = graph.relevance_of(other.0).reward();
            return Ord::cmp(&this_rew, &that_rew);
        }

        let mut this_iter = self.1.iter();
        let mut that_iter = other.1.iter();
        loop {
            return match (this_iter.next(), that_iter.next()) {
                (None, None) => break,
                (Some(&u), Some(&v)) if u == v => continue,
                (Some(&u), Some(&v)) => Ord::cmp(
                    &graph.relevance_of(u).reward(),
                    &graph.relevance_of(v).reward(),
                ),
                (Some(&u), None) => match graph.relevance_of(u).player() {
                    Player::P0 => Ordering::Greater,
                    Player::P1 => Ordering::Less,
                },
                (None, Some(&u)) => match graph.relevance_of(u).player() {
                    Player::P0 => Ordering::Less,
                    Player::P1 => Ordering::Greater,
                },
            };
        }

        // Compare the number of nodes visited before most relevant vertex of the loop
        match graph.relevance_of(self.0).player() {
            // If P0 is winning a shorter path is better (order is reversed, less is greater).
            Player::P0 => Ord::cmp(&self.2, &other.2).reverse(),
            // If P0 is losing a longer path is better (order is normal).
            Player::P1 => Ord::cmp(&self.2, &other.2),
        }
    }
}

pub fn valuation(graph: &Graph) -> Vec<PlayProfile> {
    // TODO: Bitset or something similar?
    let mut evaluated: Set<NodeId> = Set::new();
    let mut profiles = vec![PlayProfile::default(); graph.node_count()];

    // Iterate by reward order, i.e. first nodes that are more in favour of player 1.
    // At each iteration we will try to fix all the loops that go through w, if w is not
    // already part of one.
    for w in graph.nodes_sorted_by_reward() {
        // Ignore already evaluated nodes
        if evaluated.contains(&w) {
            continue;
        }

        // Consider only predecessors that weren't "removed".
        let predecessors_of = |n| graph.predecessors_of(n).filter(|v| !evaluated.contains(v));

        // Find all nodes v <= w that can reach w
        let w_relevance = graph.relevance_of(w);
        let (_, reach_set) = reach(w, |u| {
            // TODO: is this really necessary?
            predecessors_of(u).filter(|&v| graph.relevance_of(v) <= w_relevance)
        });

        // If w cannot reach itself without going through nodes with higher priority
        // it cannot be the most relevant node of a loop.
        if !reach_set.contains(&w) {
            continue;
        }

        // Find all nodes that can reach w.
        let (mut k_nodes, k_set) = reach(w, predecessors_of);

        // Subevaluation: force all cycles that contain w to happen,
        // with the best path possible.
        subevaluation(graph, w, &mut k_nodes, &k_set, &mut profiles, &evaluated);

        // Equivalent to removing edges from K to V \ K,
        // as it will make sure they will never get explored again.
        for &v in &k_nodes {
            evaluated.insert(v);
        }
    }

    profiles
}

/// A restricted graph without some nodes or edges.
struct RestrictedGraph<'a> {
    /// The base graph
    base: &'a Graph,
    /// List of nodes that are in the restricted graph (for fast iteration)
    k_nodes: &'a [NodeId],
    /// Set of nodes that are in the restricted graph (for filtering/checking)
    k_set: &'a Set<NodeId>,
    /// Edges removed from the graph
    removed_edges: Set<(NodeId, NodeId)>,
    /// Number of outgoing edges removed from each node
    removed_successors_count: NodeMap<usize>,
}

impl<'a> RestrictedGraph<'a> {
    fn predecessors_of(&self, v: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.base
            .predecessors_of(v)
            .filter(|&u| self.k_set.contains(&u))
            .filter(move |&u| !self.removed_edges.contains(&(u, v)))
    }

    fn successors_count_of(&self, v: NodeId) -> usize {
        // TODO: this need to account for edges that go outside K
        self.base.successors_count_of(v) - self.removed_successors_count.get(&v).unwrap_or(&0)
    }

    fn all_successors_of(&self, v: NodeId) -> impl Iterator<Item = NodeId> + 'a {
        self.base.successors_of(v)
    }

    fn remove_edge(&mut self, v: NodeId, u: NodeId) {
        if self.removed_edges.insert((v, u)) {
            *self.removed_successors_count.entry(v).or_insert(0) += 1;
        }
    }

    fn relevance_of(&self, v: NodeId) -> Relevance {
        self.base.relevance_of(v)
    }
}

fn subevaluation(
    graph: &Graph,
    w: NodeId,
    k_nodes: &mut [NodeId],
    k_set: &Set<NodeId>,
    profiles: &mut [PlayProfile],
    evaluated: &Set<NodeId>,
) {
    // Sort K by relevance, for the loop later on.
    k_nodes.sort_by_key(|&v| graph.relevance_of(v));

    let mut graph = RestrictedGraph {
        base: graph,
        k_nodes,
        k_set,
        removed_edges: Set::new(),
        removed_successors_count: NodeMap::new(),
    };

    let w_relevance = graph.relevance_of(w);

    // All these nodes will be part of cycles that contain w as most relevant node.
    for &v in &*graph.k_nodes {
        profiles[v.0].0 = w;
    }

    // Iterate over K with descending relevance order for those nodes that have
    // higher relevance than w
    for &u in graph.k_nodes.iter().rev() {
        if graph.relevance_of(u) <= w_relevance {
            break;
        }

        match graph.relevance_of(u).player() {
            Player::P0 => prevent_paths(&mut graph, w, u, profiles),
            Player::P1 => force_paths(&mut graph, w, u, profiles),
        }
    }

    // Extra: sort the nodes in the profile by their relevance, as that will help
    // when comparing profiles.
    for v in &*graph.k_nodes {
        profiles[v.0]
            .1
            .sort_by_key(|&n| Reverse(graph.relevance_of(n)));
    }

    match graph.relevance_of(w).player() {
        Player::P0 => set_maximal_distances(&mut graph, w, profiles),
        Player::P1 => set_minimal_distances(&mut graph, w, profiles),
    }
}

/// Prevent any path that can go through u from doing so.
fn prevent_paths(graph: &mut RestrictedGraph, w: NodeId, u: NodeId, profiles: &mut [PlayProfile]) {
    // Find nodes that can reach w without going through u.
    let (u_nodes, u_set) = reach(w, |n| graph.predecessors_of(n).filter(|&v| v != u));

    // Update profiles of nodes whose path must go through u.
    for &v in graph.k_nodes.iter().filter(|v| !u_set.contains(v)) {
        profiles[v.0].1.push(u);
    }

    // Remove edges that would make paths go through u when it's possible
    // to avoid it, that is edges from u_nodes U {u} to V \ U_nodes.
    for &v in u_nodes.iter().chain([&u]) {
        for next in graph.all_successors_of(v).filter(|n| !u_set.contains(n)) {
            graph.remove_edge(v, next);
        }
    }
}

/// Make any path that can go through u do so.
fn force_paths(graph: &mut RestrictedGraph, w: NodeId, u: NodeId, profiles: &mut [PlayProfile]) {
    // Find nodes that can reach u without going through w.
    let (u_nodes, u_set) = reach(u, |n| graph.predecessors_of(n).filter(|&v| v != w));

    // Update profiles of nodes whose path can go through u.
    for &v in graph.k_nodes.iter().filter(|v| u_set.contains(v)) {
        profiles[v.0].1.push(u);
    }

    // Remove edges that would make paths not go through u when it's possible
    // to do so, that is edges from u_nodes \ {u} to V \ u_nodes
    for &v in u_nodes.iter().filter(|&&v| v != u) {
        for next in graph.all_successors_of(v).filter(|n| !u_set.contains(n)) {
            graph.remove_edge(v, next);
        }
    }
}

fn reach<F, I>(start: NodeId, mut explore: F) -> (Vec<NodeId>, Set<NodeId>)
where
    F: FnMut(NodeId) -> I,
    I: IntoIterator<Item = NodeId>,
{
    let mut stack = Vec::from_iter(explore(start));
    let (mut nodes, mut set) = (Vec::new(), Set::new());

    // BFS according to explore
    while let Some(node) = stack.pop() {
        if set.insert(node) {
            nodes.push(node);
            stack.extend(explore(node));
        }
    }

    (nodes, set)
}

fn set_maximal_distances(graph: &mut RestrictedGraph, w: NodeId, profiles: &mut [PlayProfile]) {
    let mut remaining_successors = graph
        .k_nodes
        .iter()
        .map(|&v| (v, graph.successors_count_of(v)))
        .collect::<NodeMap<_>>();
    let mut queue = VecDeque::from([(w, 0)]);

    remaining_successors.remove(&w);

    while let Some((v, d)) = queue.pop_front() {
        profiles[v.0].2 = d;
        for u in graph.predecessors_of(v).filter(|&u| u != w) {
            // Decrease number of remaining successors to visit
            let remaining = remaining_successors.get_mut(&u).unwrap();
            *remaining -= 1;

            // If last was visited then add node to the queue with one more edge.
            if *remaining == 0 {
                queue.push_back((u, d + 1));
            }
        }
    }
}

fn set_minimal_distances(graph: &mut RestrictedGraph, w: NodeId, profiles: &mut [PlayProfile]) {
    let mut seen = Set::new();
    let mut queue = VecDeque::from([(w, 0)]);

    // Backward BFS
    while let Some((v, d)) = queue.pop_front() {
        if seen.insert(v) {
            profiles[v.0].2 = d;
            queue.extend(graph.predecessors_of(v).map(|u| (u, d + 1)))
        }
    }
}
