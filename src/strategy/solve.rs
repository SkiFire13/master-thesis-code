use crate::index::IndexVec;
use crate::strategy::expansion::expand;
use crate::strategy::game::{Game, GameStrategy, NodeId, NodeP0Id, NodeP1Id};
use crate::strategy::improvement::{improve, valuation, PlayProfile};
use crate::symbolic::compose::EqsFormulas;
use crate::symbolic::eq::VarId;
use crate::symbolic::formula::BasisElemId;

pub fn solve(b: BasisElemId, i: VarId, moves: EqsFormulas) -> bool {
    // Special case to ensure there's always a move possible.
    if moves.get(b, i).is_false() {
        return false;
    }

    let mut game = Game::new(b, i, moves);

    expand(&mut game, &initial_play_profiles());

    // TODO: init W0/W1

    // Select initial strategy by picking a random successor for each p0 node.
    let mut strategy = {
        let direct = game
            .p0_succs
            .iter()
            .map(|succs| succs.first().copied())
            .collect::<IndexVec<NodeP0Id, Option<NodeP1Id>>>();
        GameStrategy::from_direct(&game, direct)
    };

    loop {
        let (profiles, final_strategy) = loop {
            let (profiles, final_strategy) = valuation(&game, &strategy);
            let improved = improve(&game, &mut strategy, &profiles);

            if !improved {
                break (profiles, final_strategy);
            }
        };

        _ = (profiles, final_strategy);

        // TODO: update W0/W1
        // TODO: exit if INIT/(b, i) is in W0/W1
        // TODO: expand graph
        // TODO: expand strategy
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
