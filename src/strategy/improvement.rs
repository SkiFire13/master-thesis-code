use std::cmp::{Ordering, Reverse};
use std::collections::VecDeque;
use std::slice;

use either::Either::{Left, Right};

use crate::index::IndexVec;
use crate::strategy::game::NodeData;

use super::game::{Game, NodeId, NodeP0Id, NodeP1Id, Player, Relevance};

// TODO: Use node/vertex consistently

// Bitset or something similar?
pub type Set<T> = std::collections::BTreeSet<T>;
pub type NodeMap<T> = std::collections::HashMap<NodeId, T>;

struct Graph<'a> {
    game: &'a Game,
    strategy: &'a IndexVec<NodeP0Id, NodeP1Id>,
    inverse_strategy: IndexVec<NodeP1Id, Vec<NodeP0Id>>,
}

impl<'a> Graph<'a> {
    fn successors_of(&self, n: NodeId) -> impl Iterator<Item = NodeId> + 'a {
        match self.game.resolve(n) {
            // Successors of special nodes are only other special nodes.
            NodeData::L0 => Left(&[NodeId::W1][..]),
            NodeData::L1 => Left(&[NodeId::W0][..]),
            NodeData::W0 => Left(&[NodeId::L1][..]),
            NodeData::W1 => Left(&[NodeId::L0][..]),
            // The successor of a p0 node is the p1 node given by the strategy.
            NodeData::P0(n) => Left(slice::from_ref(&self.game.p1_ids[self.strategy[n]])),
            // The successors of a p1 node are all those recorded in the current game.
            NodeData::P1(n) => Right(self.game.p1_succs[n].iter().map(|&n| self.game.p0_ids[n])),
        }
        .map_left(|slice| slice.iter().copied())
    }

    fn predecessors_of(&self, n: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        match self.game.resolve(n) {
            // The predecessor of a L node is just the corresponding W node.
            NodeData::L0 => Left(&[NodeId::W1][..]),
            NodeData::L1 => Left(&[NodeId::W0][..]),
            // The predecessor of the W0 node is the empty P1 node
            NodeData::W0 => Left(
                self.game
                    .w0_pred()
                    .map_or(&[][..], |n| slice::from_ref(&self.game.p1_ids[n])),
            ),
            // The predecessor of the W1 node are all those P0 nodes with a false formula.
            NodeData::W1 => Right(Right(self.game.w1_preds.iter())),
            // The predecessors of a p0 node are all those recorded in the game.
            NodeData::P0(n) => Right(Left(self.game.p0_preds[n].iter())),
            // The predecessors of a p1 node are those given by the strategy.
            NodeData::P1(n) => Right(Right(self.inverse_strategy[n].iter())),
        }
        .map_left(|slice| slice.iter().copied())
        .map_right(|inner| inner.map_left(|iter| iter.map(|&n| self.game.p1_ids[n])))
        .map_right(|inner| inner.map_right(|iter| iter.map(|&n| self.game.p0_ids[n])))
    }

    fn node_count(&self) -> usize {
        self.game.nodes.len()
    }

    fn relevance_of(&self, n: NodeId) -> Relevance {
        self.game.relevance_of(n)
    }

    fn nodes_sorted_by_reward(&self) -> impl Iterator<Item = NodeId> + 'a {
        if false {
            return [].into_iter();
        }
        todo!()
    }
}

#[derive(Clone, Default)]
pub struct PlayProfile {
    /// Most relevant node of the cycle.
    pub most_relevant: NodeId,
    /// Nodes more relevant visited before the cycle, sorted by most relevant first.
    pub relevant_before: Vec<NodeId>,
    /// Number of nodes visited before the most relevant of the cycle.
    pub count_before: usize,
}

impl PlayProfile {
    pub fn cmp(&self, that: &PlayProfile, game: &Game) -> Ordering {
        // Compare the most relevant vertex of the cycle
        if self.most_relevant != that.most_relevant {
            let this_rew = game.relevance_of(self.most_relevant).reward();
            let that_rew = game.relevance_of(that.most_relevant).reward();
            return Ord::cmp(&this_rew, &that_rew);
        }

        // Compare the set of more relevant nodes visited before the cycle
        let mut this_iter = self.relevant_before.iter();
        let mut that_iter = that.relevant_before.iter();
        loop {
            // The vecs are sorted by most relevant first so this will compare the
            // most relevant of each set until one has a more relevant one or runs out of nodes.
            return match (this_iter.next(), that_iter.next()) {
                // If both ran out of nodes they are the same
                (None, None) => break,
                // Ignore when both have the same node.
                (Some(&u), Some(&v)) if u == v => continue,
                // If the nodes are different, compare their rewards
                (Some(&u), Some(&v)) => Ord::cmp(
                    &game.relevance_of(u).reward(),
                    &game.relevance_of(v).reward(),
                ),
                // If the other profile ran out of nodes, this wins if the node benefits p0
                (Some(&u), None) => match game.relevance_of(u).player() {
                    Player::P0 => Ordering::Greater,
                    Player::P1 => Ordering::Less,
                },
                // If the this profile ran out of nodes, this wins if the other node benefits p1
                (None, Some(&u)) => match game.relevance_of(u).player() {
                    Player::P0 => Ordering::Less,
                    Player::P1 => Ordering::Greater,
                },
            };
        }

        // Compare the number of nodes visited before most relevant vertex of the loop
        match game.relevance_of(self.most_relevant).player() {
            // If P0 is winning a shorter path is better (order is reversed, less is greater).
            Player::P0 => Ord::cmp(&self.count_before, &that.count_before).reverse(),
            // If P0 is losing a longer path is better (order is normal).
            Player::P1 => Ord::cmp(&self.count_before, &that.count_before),
        }
    }
}

pub fn valuation(
    game: &Game,
    strategy: &IndexVec<NodeP0Id, NodeP1Id>,
) -> IndexVec<NodeId, PlayProfile> {
    // Build graph with p0 moves restricted to the given strategy.
    let mut inverse_strategy = IndexVec::from(vec![Vec::new(); game.p1_set.len()]);
    for (n0, &n1) in strategy.iter().enumerate() {
        inverse_strategy[n1].push(NodeP0Id(n0));
    }
    let graph = &Graph { game, strategy, inverse_strategy };

    let mut evaluated = Set::new();
    let mut profiles = IndexVec::from(vec![PlayProfile::default(); graph.node_count()]);

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
        subevaluation(graph, w, &mut k_nodes, &k_set, &mut profiles);

        // Equivalent to removing edges from K to V \ K,
        // as it will make sure they will never get explored again.
        evaluated.extend(k_nodes);
    }

    profiles
}

/// A restricted graph without some nodes or edges.
struct RestrictedGraph<'a> {
    /// The base graph
    base: &'a Graph<'a>,
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
        // Take all successors and consider only those in the original K.
        // Then remove from the count those edges that were removed.
        self.base
            .successors_of(v)
            .filter(|u| self.k_set.contains(u))
            .count()
            - self.removed_successors_count.get(&v).unwrap_or(&0)
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
    profiles: &mut IndexVec<NodeId, PlayProfile>,
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
        profiles[v].most_relevant = w;
    }

    // Iterate over K with descending relevance order, considering only those
    // nodes that have higher relevance than w.
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
    for &v in &*graph.k_nodes {
        profiles[v]
            .relevant_before
            .sort_by_key(|&n| Reverse(graph.relevance_of(n)));
    }

    // Depending on the player favoured by w maximize or minimize the distances.
    match graph.relevance_of(w).player() {
        Player::P0 => set_maximal_distances(&mut graph, w, profiles),
        Player::P1 => set_minimal_distances(&mut graph, w, profiles),
    }
}

/// Prevent any path that can go through u from doing so.
fn prevent_paths(
    graph: &mut RestrictedGraph,
    w: NodeId,
    u: NodeId,
    profiles: &mut IndexVec<NodeId, PlayProfile>,
) {
    // Find nodes that can reach w without going through u.
    let (u_nodes, u_set) = reach(w, |n| graph.predecessors_of(n).filter(|&v| v != u));

    // Update profiles of nodes whose path must go through u.
    for &v in graph.k_nodes.iter().filter(|v| !u_set.contains(v)) {
        profiles[v].relevant_before.push(u);
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
fn force_paths(
    graph: &mut RestrictedGraph,
    w: NodeId,
    u: NodeId,
    profiles: &mut IndexVec<NodeId, PlayProfile>,
) {
    // Find nodes that can reach u without going through w.
    let (u_nodes, u_set) = reach(u, |n| graph.predecessors_of(n).filter(|&v| v != w));

    // Update profiles of nodes whose path can go through u.
    for &v in graph.k_nodes.iter().filter(|v| u_set.contains(v)) {
        profiles[v].relevant_before.push(u);
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

fn set_maximal_distances(
    graph: &mut RestrictedGraph,
    w: NodeId,
    profiles: &mut IndexVec<NodeId, PlayProfile>,
) {
    let mut remaining_successors = graph
        .k_nodes
        .iter()
        .map(|&v| (v, graph.successors_count_of(v)))
        .collect::<NodeMap<_>>();
    let mut queue = VecDeque::from([(w, 0)]);

    while let Some((v, d)) = queue.pop_front() {
        profiles[v].count_before = d;

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

fn set_minimal_distances(
    graph: &mut RestrictedGraph,
    w: NodeId,
    profiles: &mut IndexVec<NodeId, PlayProfile>,
) {
    let mut seen = Set::new();
    let mut queue = VecDeque::from([(w, 0)]);

    // Backward BFS
    while let Some((v, d)) = queue.pop_front() {
        if seen.insert(v) {
            profiles[v].count_before = d;
            queue.extend(graph.predecessors_of(v).map(|u| (u, d + 1)))
        }
    }
}

// TODO: improve function
