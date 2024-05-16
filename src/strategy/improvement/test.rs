use std::cmp::Reverse;

use indexmap::IndexSet;

use crate::index::IndexVec;
use crate::strategy::game::{NodeId, NodeP0Id, NodeP1Id, Player, Relevance};

use super::valuation::{valuation, Strategy, ValuationGraph};
use super::{GetRelevance, NodeMap};

#[derive(Default)]
struct TestGame {
    relevance: IndexVec<NodeId, usize>,
    p0_nodes: IndexVec<NodeP0Id, NodeId>,
    p1_nodes: IndexVec<NodeP1Id, NodeId>,
    nodes_p: IndexVec<NodeId, PlayerNode>,
    successors: IndexVec<NodeId, Vec<NodeId>>,
    predecessors: IndexVec<NodeId, Vec<NodeId>>,
    nodes_by_reward: Vec<NodeId>,
}

struct TestStrategy {
    direct: NodeMap<NodeId>,
    inverse: NodeMap<Vec<NodeId>>,
}

enum PlayerNode {
    P0(NodeP0Id),
    P1(NodeP1Id),
}

impl GetRelevance for TestGame {
    fn relevance_of(&self, u: NodeId) -> Relevance {
        Relevance(self.relevance[u], u)
    }
}

impl ValuationGraph for TestGame {
    fn node_count(&self) -> usize {
        self.nodes_p.len()
    }

    fn player(&self, n: NodeId) -> Player {
        match self.nodes_p[n] {
            PlayerNode::P0(_) => Player::P0,
            PlayerNode::P1(_) => Player::P1,
        }
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

    fn get(&self, n: NodeId, _: &Self::Graph) -> NodeId {
        self.direct[&n]
    }

    fn get_inverse(&self, n: NodeId, _: &Self::Graph) -> impl Iterator<Item = NodeId> {
        self.inverse
            .get(&n)
            .into_iter()
            .flat_map(|preds| preds.iter().copied())
    }
}

fn parse_test(test: &str) -> TestGame {
    let mut game = TestGame::default();
    for line in test.lines().skip(1) {
        let (_, rest) = line.split_once(' ').unwrap();
        let (relevance, rest) = rest.split_once(' ').unwrap();
        let (player, successors) = rest.split_once(' ').unwrap();

        let n = game.relevance.push(relevance.parse().unwrap());
        match player {
            "0" => _ = game.nodes_p.push(PlayerNode::P0(game.p0_nodes.push(n))),
            "1" => _ = game.nodes_p.push(PlayerNode::P1(game.p1_nodes.push(n))),
            _ => panic!(),
        }
        let successors = successors.strip_suffix(';').unwrap().split(',');
        game.successors
            .push(successors.map(|i| NodeId(i.parse().unwrap())).collect());
    }

    game.predecessors
        .resize_with(game.successors.len(), Vec::new);
    for (n, succ) in game.successors.iter().enumerate() {
        for &m in succ {
            game.predecessors[m].push(NodeId(n));
        }
    }

    game.nodes_by_reward = (0..game.relevance.len()).map(NodeId).collect::<Vec<_>>();
    game.nodes_by_reward
        .sort_unstable_by_key(|&n| Relevance(game.relevance[n], n).reward());

    game
}

fn run_valuation_test(game: &TestGame) {
    let direct_strategy = game
        .p0_nodes
        .iter()
        .map(|&n| (n, *game.successors[n].last().unwrap()))
        .collect::<NodeMap<_>>();
    let mut inverse_strategy = NodeMap::new();
    for (&n, &m) in direct_strategy.iter() {
        inverse_strategy.entry(m).or_insert_with(Vec::new).push(n);
    }
    let strategy = TestStrategy { direct: direct_strategy, inverse: inverse_strategy };

    let (profiles, final_strategy) = valuation(game, &strategy);

    for n in (0..game.nodes_p.len()).map(NodeId) {
        let profile = &profiles[n];
        let next = final_strategy[n];

        // Check that the final strategy is consistent with the given p0 strategy
        // and the successors of p1.
        match game.nodes_p[n] {
            PlayerNode::P0(_) => assert_eq!(strategy.direct[&n], next),
            PlayerNode::P1(_) => assert!(game.successors[n].contains(&next)),
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
    vb059,
}
