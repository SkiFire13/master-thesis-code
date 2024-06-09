use crate::index::IndexedVec;
use crate::strategy::expansion::expand;
use crate::strategy::game::{Game, GameStrategy, NodeId};
use crate::strategy::improvement::{improve, valuation, PlayProfile};
use crate::symbolic::compose::EqsFormulas;
use crate::symbolic::eq::VarId;
use crate::symbolic::formula::BasisElemId;

use super::escape::update_winning_sets;
use super::game::NodeP0Id;

pub fn solve(b: BasisElemId, i: VarId, moves: EqsFormulas) -> bool {
    // Special case to ensure there's always a move possible.
    if moves.get(b, i).is_false() {
        return false;
    }

    let mut game = Game::new(b, i, moves);
    let mut strategy = GameStrategy::new();

    // Initial expansion
    expand(&mut game, &initial_play_profiles());
    strategy.expand(&game);

    loop {
        // Try to improve while possible
        let (profiles, final_strategy) = loop {
            let (profiles, final_strategy) = valuation(&game, &strategy);
            let improved = improve(&game, &mut strategy, &profiles);

            if !improved {
                break (profiles, final_strategy);
            }
        };

        // Update definitely winning/losing nodes.
        update_winning_sets(&mut game, &profiles, &final_strategy);

        match () {
            _ if game.p0.w0.contains(&NodeP0Id::INIT) => return true,
            _ if game.p0.w1.contains(&NodeP0Id::INIT) => return false,
            _ => {}
        }

        // We still don't know whether the initial node is definitely winning/losing
        // so expand again the graph.
        expand(&mut game, &profiles);
        strategy.expand(&game);
    }
}

fn initial_play_profiles() -> IndexedVec<NodeId, PlayProfile> {
    let w0 =
        || PlayProfile { most_relevant: NodeId::W0, relevant_before: Vec::new(), count_before: 0 };
    let w1 =
        || PlayProfile { most_relevant: NodeId::W1, relevant_before: Vec::new(), count_before: 0 };

    // Corresponding nodes are: W0, L0, W1, L1, INIT
    IndexedVec::from(vec![w0(), w1(), w1(), w0(), w1()])
}
