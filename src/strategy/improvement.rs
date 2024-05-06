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

    fn relevance_of(&self, n: NodeId) -> Relevance {
        todo!()
    }

    fn nodes_sorted_by_reward(&self) -> impl Iterator<Item = NodeId> {
        todo!();
        [].into_iter()
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

        // TODO: compare the two lists
        //  - if they have two elements where they differ, compare their reward;
        //  - if one has strictly more elements than the other, look at the winning player in that element.
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

        let predecessors_of = |n| graph.predecessors_of(n).filter(|v| !evaluated.contains(v));

        // Find all nodes v <= w that can reach w
        let w_relevance = graph.relevance_of(w);
        let (_, reach_set) = reach(w, |u| {
            // TODO: is this really necessary?
            predecessors_of(u).filter(|&v| graph.relevance_of(v) <= w_relevance)
        });

        // If w cannot reach itself it cannot create a loop, ignore it.
        if !reach_set.contains(&w) {
            continue;
        }

        // Find all nodes that can reach w.
        let (mut k_nodes, _) = reach(w, predecessors_of);

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

/// A restricted graph without some nodes or edges.
struct RestrictedGraph<'a> {
    /// The base graph
    base: &'a Graph,
    /// Already evaluated nodes, thus excluded from the graph
    evaluated: &'a Set<NodeId>,
    /// Edges removed from the graph
    removed_edges: Set<(NodeId, NodeId)>,
}

impl<'a> std::ops::Deref for RestrictedGraph<'a> {
    type Target = &'a Graph;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

fn subevaluation(
    graph: &Graph,
    w: NodeId,
    k_nodes: &mut [NodeId],
    profiles: &mut [PlayProfile],
    evaluated: &Set<NodeId>,
) {
    let mut graph = RestrictedGraph { base: graph, evaluated, removed_edges: Set::new() };

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

        match graph.relevance_of(u).player() {
            Player::P0 => prevent_paths(&mut graph, w, u, k_nodes, profiles),
            Player::P1 => force_paths(&mut graph, w, u, k_nodes, profiles),
        }
    }

    // Extra: sort the nodes in the profile by their relevance, as that will help
    // when comparing profiles.
    for v in &*k_nodes {
        profiles[v.0]
            .1
            .sort_by_key(|&n| Reverse(graph.relevance_of(n)));
    }

    match graph.relevance_of(w).player() {
        Player::P0 => {} // TODO: maximal_distances
        Player::P1 => {} // TODO: minimal_distances
    }
}

/// Prevent any path that can go through u from doing so.
fn prevent_paths(
    graph: &mut RestrictedGraph,
    w: NodeId,
    u: NodeId,
    k_nodes: &[NodeId],
    profiles: &mut [PlayProfile],
) {
    // Find nodes reachable from w in the graph excluding u.
    let (u_nodes, u_set) = reach(w, |n| {
        let removed_edges = &graph.removed_edges;
        graph
            .predecessors_of(n)
            .filter(|&v| !graph.evaluated.contains(&v))
            .filter(move |&v| !removed_edges.contains(&(v, n)))
            .filter(|&v| v != u)
    });

    // Update profiles of those path that must go through u
    for &v in k_nodes.iter().filter(|v| !u_set.contains(v)) {
        profiles[v.0].1.push(u);
    }

    // Remove edges from u_nodes U {u} to V \ U_nodes
    for &v in u_nodes.iter().chain([&u]) {
        for next in graph.successors_of(v).filter(|next| !u_set.contains(next)) {
            graph.removed_edges.insert((v, next));
        }
    }
}

/// Make any path that can go through u do so.
fn force_paths(
    graph: &mut RestrictedGraph,
    w: NodeId,
    u: NodeId,
    k_nodes: &[NodeId],
    profiles: &mut [PlayProfile],
) {
    // Find nodes reachable from w in the graph excluding u.
    let (u_nodes, u_set) = reach(u, |n| {
        let removed_edges = &graph.removed_edges;
        graph
            .predecessors_of(n)
            .filter(|&v| !graph.evaluated.contains(&v))
            .filter(move |&v| !removed_edges.contains(&(v, n)))
            .filter(|&v| v != w)
    });

    // Update profiles of those path that can go through u
    for &v in k_nodes.iter().filter(|v| u_set.contains(v)) {
        profiles[v.0].1.push(u);
    }

    // Remove edges from u_nodes \ {u} to V \ u_nodes
    for &v in u_nodes.iter().filter(|&&v| v != u) {
        for next in graph.successors_of(v).filter(|next| !u_set.contains(next)) {
            graph.removed_edges.insert((v, next));
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

    while let Some(node) = stack.pop() {
        if set.insert(node) {
            nodes.push(node);
            stack.extend(explore(node));
        }
    }

    (nodes, set)
}
