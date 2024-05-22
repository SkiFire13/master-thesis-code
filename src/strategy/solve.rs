use crate::index::IndexVec;
use crate::strategy::expansion::expand;
use crate::strategy::game::{Game, GameStrategy, NodeP0Id, NodeP1Id};
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

    // TODO: This play profile vec is wrong.
    expand(&mut game, &IndexVec::from(vec![PlayProfile::default()]));
    // TODO: init/update W0/W1

    // Select initial strategy by picking a random successor for each p0 node.
    let mut strategy = {
        let direct = game
            .p0_succs
            .iter()
            .map(|succs| succs.first().copied())
            .collect::<IndexVec<NodeP0Id, Option<NodeP1Id>>>();
        GameStrategy::from_direct(&game, direct)
    };

    // TODO: (b, i) not in W0 or W1
    while false {
        let (profiles, _final_strategy) = valuation(&game, &strategy);

        if improve(&game, &mut strategy, &profiles) {
            // TODO: update W0/W1 (is it possible?)
        } else {
            // expand graph
            // expand strategy
        }
    }

    todo!("is (b, i) in W0?")
}
