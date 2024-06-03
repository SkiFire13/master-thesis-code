use std::cmp::Reverse;

use indexmap::IndexSet;

use crate::index::IndexedVec;
use crate::strategy::game::{NodeId, Player, Relevance};
use crate::strategy::NodeMap;

use super::valuation::{valuation, Strategy, ValuationGraph};
use super::GetRelevance;

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

impl ValuationGraph for TestGame {
    fn node_count(&self) -> usize {
        self.players.len()
    }

    fn player(&self, n: NodeId) -> Player {
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

fn parse_test(test: &str) -> TestGame {
    let mut game = TestGame::default();
    for line in test.lines().skip(1) {
        let (_, rest) = line.split_once(' ').unwrap();
        let (relevance, rest) = rest.split_once(' ').unwrap();
        let (player, rest) = rest.split_once(' ').unwrap();
        let (successors, _) = rest.split_once([' ', ';']).unwrap();

        game.relevance.push(relevance.parse().unwrap());
        match player {
            "0" => _ = game.players.push(Player::P0),
            "1" => _ = game.players.push(Player::P1),
            _ => panic!(),
        }
        let successors = successors.split(',').map(|i| NodeId(i.parse().unwrap())).collect();
        game.successors.push(successors);
    }

    game.predecessors.resize_with(game.successors.len(), Vec::new);
    for (n, succ) in game.successors.enumerate() {
        for &m in succ {
            game.predecessors[m].push(n);
        }
    }

    game.nodes_by_reward = (0..game.relevance.len()).map(NodeId).collect::<Vec<_>>();
    game.nodes_by_reward
        .sort_unstable_by_key(|&node| Relevance { priority: game.relevance[node], node }.reward());

    game
}

fn run_valuation_test(game: &TestGame) {
    let direct_strategy = game
        .players
        .enumerate()
        .filter(|(_, &p)| p == Player::P0)
        .map(|(n, _)| (n, *game.successors[n].last().unwrap()))
        .collect::<NodeMap<_>>();
    let mut inverse_strategy = NodeMap::new();
    for (&n, &m) in direct_strategy.iter() {
        inverse_strategy.entry(m).or_insert_with(Vec::new).push(n);
    }
    let strategy = TestStrategy { direct: direct_strategy, inverse: inverse_strategy };

    let (profiles, final_strategy) = valuation(game, &strategy);

    for n in (0..game.players.len()).map(NodeId) {
        let profile = &profiles[n];
        let next = final_strategy[n];

        assert!(next < NodeId(game.players.len()));

        // Check that the final strategy is consistent with the given p0 strategy
        // and the successors of p1.
        match game.players[n] {
            Player::P0 => assert_eq!(strategy.direct[&n], next),
            Player::P1 => assert!(game.successors[n].contains(&next), "n={n:?} succ={next:?}"),
        }

        // Play the game with the final strategy until a node is seen twice.
        let (mut curr, mut seen) = (n, IndexSet::new());
        while seen.insert(curr) {
            curr = final_strategy[curr];
        }

        let rel_of = |n| game.relevance_of(n);

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
                let input = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/test/data/", stringify!($name)));
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
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/test/data/");
    for e in std::fs::read_dir(dir).unwrap() {
        let e = e.unwrap();

        let name = e.file_name().into_string().unwrap();
        if !name.starts_with("vb") {
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
