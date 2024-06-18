use std::rc::Rc;

use crate::index::IndexedVec;
use crate::strategy::expansion::expand;
use crate::strategy::game::{Game, GameStrategy, NodeId, NodeP1Id};
use crate::strategy::improvement::{improve, valuation, PlayProfile};
use crate::symbolic::compose::EqsFormulas;
use crate::symbolic::eq::VarId;
use crate::symbolic::formula::BasisElemId;

use super::escape::update_winning_sets;
use super::game::NodeP0Id;

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

    loop {
        // Initially this will perform the initial expansion and set a proper successor for INIT.
        // Later on it will expand the graph, potentially running `update_winning_sets`.
        expand(&mut game, &mut profiles, &mut final_strategy, &mut strategy);

        match () {
            _ if game.p0.w0.contains(&NodeP0Id::INIT) => return true,
            _ if game.p0.w1.contains(&NodeP0Id::INIT) => return false,
            _ => {}
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
        match () {
            _ if game.p0.w0.contains(&NodeP0Id::INIT) => return true,
            _ if game.p0.w1.contains(&NodeP0Id::INIT) => return false,
            _ => {}
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
