use std::collections::HashMap;

use solver::index::IndexedVec;
use solver::strategy::Player;
use solver::symbolic::compose::FunsFormulas;
use solver::symbolic::eq::{Expr, FixEq, FixType, VarId};

use crate::ParityGame;

pub fn parity_game_to_fix(
    pg: &ParityGame,
) -> (IndexedVec<VarId, FixEq>, FunsFormulas, HashMap<usize, VarId>) {
    let mut sorted_nodes = pg.nodes.iter().collect::<IndexedVec<VarId, _>>();
    sorted_nodes.sort_by_key(|n| n.relevance);

    let node_id_to_var_id =
        sorted_nodes.enumerate().map(|(var_id, n)| (n.id, var_id)).collect::<HashMap<_, _>>();

    let eqs = sorted_nodes
        .iter()
        .map(|n| {
            let fix_type = match n.relevance % 2 {
                0 => FixType::Max,
                _ => FixType::Min,
            };

            let children = n.successors.iter().map(|n| Expr::Var(node_id_to_var_id[&n])).collect();
            let expr = match n.player {
                Player::P0 => Expr::Or(children),
                Player::P1 => Expr::And(children),
            };

            FixEq { fix_type, expr }
        })
        .collect();

    let funs_formulas = FunsFormulas::new(IndexedVec::new());

    (eqs, funs_formulas, node_id_to_var_id)
}
