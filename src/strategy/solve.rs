use crate::strategy::expansion::expand;
use crate::strategy::game::{Game, NodeP1Id};
use crate::symbolic::compose::EqsFormulas;
use crate::symbolic::eq::VarId;
use crate::symbolic::formula::BasisId;

pub fn solve(b: BasisId, i: VarId, moves: EqsFormulas) -> bool {
    // Special case to ensure there's always a move possible.
    if moves.get(b, i).is_false() {
        return false;
    }

    let mut game = Game::new(b, i, moves);

    expand(&mut game);
    // TODO: init/update W0/W1

    let mut strategy = vec![NodeP1Id(0); game.nodes_p0.len()];
    // TODO: select initial strategy

    while todo!("(b, i) not in W0 or W1") {
        // TODO: valuation

        // TODO: try to improve valuation

        // TODO: update W0/W1 (when?)

        // TODO: expand if improved enough + expand strategy
    }

    todo!("is (b, i) in W0?")
}
