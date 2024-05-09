use crate::index::IndexVec;
use crate::strategy::expansion::expand;
use crate::strategy::game::{Game, NodeP0Id, NodeP1Id};
use crate::strategy::improvement::{improve, valuation, PlayProfile};
use crate::symbolic::compose::EqsFormulas;
use crate::symbolic::eq::VarId;
use crate::symbolic::formula::BasisId;

pub fn solve(b: BasisId, i: VarId, moves: EqsFormulas) -> bool {
    // Special case to ensure there's always a move possible.
    if moves.get(b, i).is_false() {
        return false;
    }

    let mut game = Game::new(b, i, moves);

    // TODO: This play profile vec is wrong.
    expand(&mut game, &IndexVec::from(vec![PlayProfile::default()]));
    // TODO: init/update W0/W1

    let mut strategy = IndexVec::from(vec![NodeP1Id(0); game.p0_set.len()]);
    // TODO: select initial strategy

    // TMP to make it compile
    let _: IndexVec<NodeP0Id, NodeP1Id> = strategy;

    while todo!("(b, i) not in W0 or W1") {
        let profiles = valuation(&game, &strategy);

        if improve(&game, &mut strategy, &profiles) {
            // TODO: update W0/W1 (is it possible?)
        } else {
            // expand graph
            // expand strategy
        }
    }

    todo!("is (b, i) in W0?")
}
