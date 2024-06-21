use std::cmp::Reverse;

use indexmap::IndexSet;

use crate::index::{AsIndex, IndexedVec};

use super::{
    valuation, GetRelevance, NodeId, NodeMap, ParityGraph, PlayProfile, Player, Relevance, Strategy,
};

#[derive(Default)]
struct TestGame {
    relevance: IndexedVec<NodeId, usize>,
    players: IndexedVec<NodeId, Player>,
    successors: IndexedVec<NodeId, Vec<NodeId>>,
    predecessors: IndexedVec<NodeId, Vec<NodeId>>,
    nodes_by_reward: Vec<NodeId>,
}

struct TestStrategy {
    direct: NodeMap<NodeId>,
    inverse: NodeMap<Vec<NodeId>>,
}

impl GetRelevance for TestGame {
    fn relevance_of(&self, node: NodeId) -> Relevance {
        Relevance { priority: self.relevance[node], node }
    }
}

impl ParityGraph for TestGame {
    fn node_count(&self) -> usize {
        self.players.len()
    }

    fn player_of(&self, n: NodeId) -> Player {
        self.players[n]
    }

    fn successors_of(&self, n: NodeId) -> impl Iterator<Item = NodeId> {
        self.successors[n].iter().copied()
    }

    fn predecessors_of(&self, n: NodeId) -> impl Iterator<Item = NodeId> {
        self.predecessors[n].iter().copied()
    }

    fn nodes_sorted_by_reward(&self) -> impl Iterator<Item = NodeId> {
        self.nodes_by_reward.iter().copied()
    }
}

impl Strategy for TestStrategy {
    type Graph = TestGame;

    fn iter(&self, _: &Self::Graph) -> impl Iterator<Item = (NodeId, NodeId)> {
        self.direct.iter().map(|(&n, &m)| (n, m))
    }

    fn get_direct(&self, n: NodeId, _: &Self::Graph) -> NodeId {
        self.direct[&n]
    }

    fn get_inverse(&self, n: NodeId, _: &Self::Graph) -> impl Iterator<Item = NodeId> {
        self.inverse.get(&n).into_iter().flat_map(|preds| preds.iter().copied())
    }
}

fn parse_test(source: &str) -> TestGame {
    pub struct Node {
        pub id: usize,
        pub relevance: usize,
        pub player: Player,
        pub successors: Vec<usize>,
    }

    let nodes = source
        .lines()
        .skip(1)
        .map(|line| {
            let rest = line.strip_suffix(';').unwrap();
            let (id, rest) = rest.split_once(' ').unwrap();
            let (relevance, rest) = rest.split_once(' ').unwrap();
            let (player, rest) = rest.split_once(' ').unwrap();
            let (successors, _) = rest.split_once(' ').unwrap_or((rest, ""));

            let id = id.parse().unwrap();
            let relevance = relevance.parse().unwrap();
            let player = if player == "0" { Player::P0 } else { Player::P1 };
            let successors = successors.split(',').map(|s| s.parse().unwrap()).collect::<Vec<_>>();
            Node { id, relevance, player, successors }
        })
        .collect::<Vec<_>>();

    let relevance = nodes.iter().map(|n| n.relevance).collect::<IndexedVec<_, _>>();
    let players = nodes.iter().map(|n| n.player).collect::<IndexedVec<_, _>>();

    let mut successors = (0..nodes.len()).map(|_| Vec::new()).collect::<IndexedVec<_, _>>();
    let mut predecessors = (0..nodes.len()).map(|_| Vec::new()).collect::<IndexedVec<_, _>>();
    for n in &nodes {
        for &s in &n.successors {
            let (n, s) = (NodeId(n.id), NodeId(s));
            successors[n].push(s);
            predecessors[s].push(n);
        }
    }

    let mut nodes_by_reward = (0..relevance.len()).map(NodeId).collect::<Vec<_>>();
    nodes_by_reward
        .sort_unstable_by_key(|&node| Relevance { priority: relevance[node], node }.reward());

    TestGame { relevance, players, successors, predecessors, nodes_by_reward }
}

fn run_valuation_test(game: &TestGame) {
    let direct_strategy = game
        .players
        .enumerate()
        .filter(|(_, &p)| p == Player::P0)
        .map(|(n, _)| (n, *game.successors[n].last().unwrap()))
        .collect::<NodeMap<_>>();
    let mut inverse_strategy = NodeMap::default();
    for (&n, &m) in direct_strategy.iter() {
        inverse_strategy.entry(m).or_insert_with(Vec::new).push(n);
    }
    let strategy = TestStrategy { direct: direct_strategy, inverse: inverse_strategy };

    let (profiles, final_strategy) = valuation(game, &strategy);

    verify_valuation(&strategy, &profiles, &final_strategy, &game);
}

pub fn verify_valuation<S: Strategy>(
    strategy: &S,
    profiles: &IndexedVec<NodeId, PlayProfile>,
    final_strategy: &IndexedVec<NodeId, NodeId>,
    graph: &S::Graph,
) {
    for n in (0..graph.node_count()).map(NodeId) {
        let profile = &profiles[n];
        let next = final_strategy[n];

        debug_assert!(next.to_usize() < graph.node_count());

        // Check that the final strategy is consistent with the given p0 strategy
        // and the successors of p1.
        match graph.player_of(n) {
            Player::P0 => assert_eq!(strategy.get_direct(n, graph), next),
            Player::P1 => assert!(graph.successors_of(n).any(|n| n == next), "{n:?} {next:?}"),
        }

        // Play the game with the final strategy until a node is seen twice.
        let (mut curr, mut seen) = (n, IndexSet::new());
        while seen.insert(curr) {
            curr = final_strategy[curr];
        }

        let rel_of = |n| graph.relevance_of(n);

        // Find the start of the loop.
        let start = seen.get_index_of(&curr).unwrap();
        // Find the most relevant node of the loop.
        let most_relevant = *seen[start..].iter().max_by_key(|&&n| rel_of(n)).unwrap();
        // The index of the most relevant node in the sequence is also the number of nodes visited before.
        let count_before = seen.get_index_of(&most_relevant).unwrap();
        // Gather the nodes more relevant than the most relevant that are seen before it, also sort them.
        let mut relevant_before = seen[..count_before]
            .iter()
            .copied()
            .filter(|&n| rel_of(n) > rel_of(most_relevant))
            .collect::<Vec<_>>();
        relevant_before.sort_by_key(|&n| Reverse(rel_of(n)));

        // Check they are correct.
        assert_eq!(most_relevant, profile.most_relevant, "Node {n:?}");
        assert_eq!(relevant_before, profile.relevant_before, "Node {n:?}");
        assert_eq!(count_before, profile.count_before, "Node {n:?}");
    }
}

macro_rules! declare_test {
    ($($name:ident),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                let input = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../parity/tests/", stringify!($name)));
                let game = parse_test(input);
                run_valuation_test(&game);
            }
        )*
    };
}

declare_test! {
    vb001,
    vb008,
    vb013,
    vb059,
    vb133,
}

#[test]
fn all() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../parity/tests/");
    for e in std::fs::read_dir(dir).unwrap() {
        let e = e.unwrap();

        let name = e.file_name().into_string().unwrap();
        if name == ".gitignore" || name.ends_with(".sol") {
            continue;
        }

        let input = std::fs::read_to_string(e.path()).unwrap();
        let game = parse_test(&input);
        if let Err(e) = std::panic::catch_unwind(|| run_valuation_test(&game)) {
            eprintln!("Test {name} failed");
            std::panic::resume_unwind(e);
        }
    }
}
