use std::rc::Rc;

use crate::index::IndexedVec;
use crate::local::game::WinState;
use crate::strategy::{improve, valuation, NodeId, PlayProfile};
use crate::symbolic::compose::EqsFormulas;
use crate::symbolic::eq::VarId;
use crate::symbolic::formula::BasisElemId;

use super::escape::update_winning_sets;
use super::expansion::expand;
use super::game::{Game, GameStrategy, NodeP0Id, NodeP1Id};

pub fn solve(b: BasisElemId, i: VarId, moves: Rc<EqsFormulas>) -> bool {
    // Special case to ensure there's always a move possible.
    if moves.get(b, i).is_false() {
        return false;
    }

    let mut game = Game::new(b, i, moves);
    let mut strategy = GameStrategy::new();

    // Dummy initial values
    strategy.try_add(NodeP0Id::INIT, NodeP1Id::W1);
    let mut profiles = initial_play_profiles();
    let mut final_strategy = initial_final_strategy();

    let mut expand_goal = 1;

    loop {
        // Initially this will perform the initial expansion and set a proper successor for INIT.
        // Later on it will expand the graph, potentially running `update_winning_sets`.
        let solved =
            expand(&mut game, &mut profiles, &mut final_strategy, &mut strategy, expand_goal);
        expand_goal *= 2;

        // for (p0, &n0) in game.p0.ids.enumerate() {
        //     match game.p0.win[p0] {
        //         WinState::Unknown => {}
        //         WinState::Win0 => {
        //             assert_eq!(game.successors_of(n0).collect::<Vec<_>>(), vec![NodeId::L1])
        //         }
        //         WinState::Win1 => {
        //             assert_eq!(game.successors_of(n0).collect::<Vec<_>>(), vec![NodeId::W1])
        //         }
        //     }
        // }
        // for (p1, &n1) in game.p1.ids.enumerate() {
        //     match game.p1.win[p1] {
        //         WinState::Unknown => {}
        //         WinState::Win0 => {
        //             assert_eq!(game.successors_of(n1).collect::<Vec<_>>(), vec![NodeId::W0])
        //         }
        //         WinState::Win1 => {
        //             assert_eq!(game.successors_of(n1).collect::<Vec<_>>(), vec![NodeId::L0])
        //         }
        //     }
        // }

        // for (p0, succs) in game.p0.succs.enumerate() {
        //     println!(
        //         "{p0:?} [{:?}] => {succs:?} / {:?} / {}",
        //         game.p0.win[p0],
        //         game.p0.preds[p0],
        //         game.p0.moves[p0].is_exhausted()
        //     );

        //     if game.p0.win[p0] != WinState::Unknown {
        //         assert!(succs.is_empty(), "{p0:?}");
        //     }

        //     for &p1 in succs {
        //         assert!(game.p1.preds[p1].contains(&p0));
        //     }
        // }

        // for (p1, succs) in game.p1.succs.enumerate() {
        //     println!("{p1:?} [{:?}] => {succs:?} / {:?}", game.p1.win[p1], game.p1.preds[p1]);

        //     if game.p1.win[p1] != WinState::Unknown {
        //         assert!(succs.is_empty(), "{p1:?}")
        //     }

        //     for &p0 in succs {
        //         assert!(game.p0.preds[p0].contains(&p1));
        //     }
        // }

        // for (p0, preds) in game.p0.preds.enumerate() {
        //     for &p1 in preds {
        //         assert!(game.p1.succs[p1].contains(&p0));
        //     }
        // }

        // for (p1, preds) in game.p1.preds.enumerate() {
        //     for &p0 in preds {
        //         assert!(game.p0.succs[p0].contains(&p1));
        //     }
        // }

        // If expansion finished the successors and no improvement happened then see who won.
        if solved {
            update_winning_sets(&mut game, &profiles, &mut final_strategy, &mut strategy);
            match game.p0.win[NodeP0Id::INIT] {
                WinState::Win0 => return true,
                WinState::Win1 => return false,
                WinState::Unknown => {}
            }
        }

        // Try to improve while possible
        let mut improved = true;
        while improved {
            (profiles, final_strategy) = valuation(&game, &strategy);
            improved = improve(&game, &mut strategy, &profiles);
        }

        // Update definitely winning/losing nodes.
        update_winning_sets(&mut game, &profiles, &mut final_strategy, &mut strategy);

        // Check if the initial node is definitely winning/losing after the update.
        match game.p0.win[NodeP0Id::INIT] {
            WinState::Win0 => return true,
            WinState::Win1 => return false,
            WinState::Unknown => {}
        }
    }
}

fn initial_play_profiles() -> IndexedVec<NodeId, PlayProfile> {
    // Corresponding nodes are: W0, L0, W1, L1, INIT
    IndexedVec::from(vec![
        PlayProfile { most_relevant: NodeId::L1, relevant_before: Vec::new(), count_before: 1 },
        PlayProfile { most_relevant: NodeId::W1, relevant_before: Vec::new(), count_before: 1 },
        PlayProfile { most_relevant: NodeId::W1, relevant_before: Vec::new(), count_before: 0 },
        PlayProfile { most_relevant: NodeId::L1, relevant_before: Vec::new(), count_before: 0 },
        PlayProfile { most_relevant: NodeId::W1, relevant_before: Vec::new(), count_before: 1 },
    ])
}

fn initial_final_strategy() -> IndexedVec<NodeId, NodeId> {
    // Corresponding nodes are: W0, L0, W1, L1, INIT
    IndexedVec::from(vec![NodeId::L1, NodeId::W1, NodeId::L0, NodeId::W0, NodeId::W1])
}
