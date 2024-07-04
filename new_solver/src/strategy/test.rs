use std::cmp::Reverse;
use std::collections::HashMap;

use indexmap::IndexSet;

use super::{SNodeId, StrategySolver};
use crate::solver::Player;

fn parse_test(source: &str) -> StrategySolver {
    let mut game = StrategySolver::new();

    pub struct Node {
        pub id: usize,
        pub node: SNodeId,
        pub successors: Vec<usize>,
    }

    let nodes = source
        .lines()
        .skip(1)
        .map(|line| {
            let rest = line.strip_suffix(';').unwrap();
            let (id, rest) = rest.split_once(' ').unwrap();
            let (priority, rest) = rest.split_once(' ').unwrap();
            let (player, rest) = rest.split_once(' ').unwrap();
            let (successors, _) = rest.split_once(' ').unwrap_or((rest, ""));

            let id = id.parse().unwrap();

            let priority = priority.parse().unwrap();
            let player = if player == "0" { Player::P0 } else { Player::P1 };
            let node = game.add_node(player, priority);
            let node = game.node_to_snode[node];

            let successors = successors.split(',').map(|s| s.parse().unwrap()).collect::<Vec<_>>();

            Node { id, node, successors }
        })
        .collect::<Vec<_>>();

    let id_to_node = nodes.iter().map(|n| (n.id, n.node)).collect::<HashMap<_, _>>();

    for n in &nodes {
        for id in &n.successors {
            let u = n.node;
            let v = id_to_node[id];

            if game.players[u] != game.players[v] {
                game.add_edge(u, v);
            } else {
                let m = game.add_node(game.players[u].opponent(), 0);
                let m = game.node_to_snode[m];
                game.add_edge(u, m);
                game.add_edge(m, v);
            }
        }
    }

    game
}

fn run_valuation_test(game: &mut StrategySolver) {
    game.valuation();

    println!("{:?}", &*game.profiles);

    for n in game.players.indexes() {
        let profile = &game.profiles[n];
        let next = game.strategy[n];

        debug_assert!(next != SNodeId::INVALID);

        // Check that the final strategy is consistent with the given p0 strategy
        // and the successors of p1.
        match game.players[n] {
            Player::P0 => assert_eq!(game.strategy[n], next),
            Player::P1 => assert!(game.succs[n].contains(&next), "{n:?} {next:?}"),
        }

        // Play the game with the final strategy until a node is seen twice.
        let (mut curr, mut seen) = (n, IndexSet::new());
        while seen.insert(curr) {
            curr = game.strategy[curr];
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
        assert_eq!(count_before, profile.count_before, "Node {n:?} {profile:?}");
    }
}

macro_rules! declare_test {
    ($($name:ident),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                let input = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../parity/tests/", stringify!($name)));
                let mut game = parse_test(input);
                run_valuation_test(&mut game);
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
    vb016
}

#[test]
fn all() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../parity/tests/");
    let mut fails = Vec::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let e = e.unwrap();

        let name = e.file_name().into_string().unwrap();
        if name == ".gitignore" || name.ends_with(".sol") {
            continue;
        }

        let input = std::fs::read_to_string(e.path()).unwrap();
        let mut game = parse_test(&input);
        if let Err(_) =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_valuation_test(&mut game)))
        {
            fails.push(name);
        }
    }
    for name in fails {
        eprintln!("Test {name} failed");
    }
}
