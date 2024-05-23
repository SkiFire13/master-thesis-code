use crate::index::IndexVec;
use crate::strategy::expansion::expand;
use crate::strategy::game::{Game, GameStrategy, NodeId};
use crate::strategy::improvement::{improve, valuation, PlayProfile};
use crate::symbolic::compose::EqsFormulas;
use crate::symbolic::eq::VarId;
use crate::symbolic::formula::BasisElemId;

use super::escape::update_w01;
use super::game::NodeP0Id;

pub fn solve(b: BasisElemId, i: VarId, moves: EqsFormulas) -> bool {
    // Special case to ensure there's always a move possible.
    if moves.get(b, i).is_false() {
        return false;
    }

    let mut game = Game::new(b, i, moves);
    let mut strategy = GameStrategy::new();

    expand(&mut game, &initial_play_profiles());
    strategy.expand(&game);

    loop {
        let (profiles, final_strategy) = loop {
            let (profiles, final_strategy) = valuation(&game, &strategy);
            let improved = improve(&game, &mut strategy, &profiles);

            if !improved {
                break (profiles, final_strategy);
            }
        };

        update_w01(&mut game, &profiles, &final_strategy);

        // TODO: make this much less expensive
        match () {
            // The initial node is definitely winning
            _ if game.p0_w0.contains(&NodeP0Id::INIT) => return true,
            // The initial node is definitely losing
            _ if game.p0_w1.contains(&NodeP0Id::INIT) => return false,
            _ => {}
        }

        expand(&mut game, &profiles);
        strategy.expand(&game);
    }
}

fn initial_play_profiles() -> IndexVec<NodeId, PlayProfile> {
    let w0 = PlayProfile {
        most_relevant: NodeId::W0,
        relevant_before: Vec::new(),
        count_before: 0,
    };
    let w1 = PlayProfile {
        most_relevant: NodeId::W1,
        relevant_before: Vec::new(),
        count_before: 0,
    };

    IndexVec::from(vec![
        // W0
        w0.clone(),
        // L0
        w1.clone(),
        // W1
        w1.clone(),
        // L1
        w0.clone(),
        // INIT
        w1.clone(),
    ])
}
