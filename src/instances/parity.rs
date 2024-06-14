use std::collections::HashMap;

use chumsky::error::Simple;
use chumsky::primitive::{choice, just, none_of};
use chumsky::text::{self, TextParser};
use chumsky::Parser;

use crate::index::IndexedVec;
use crate::strategy::game::Player;
use crate::symbolic::compose::FunsFormulas;
use crate::symbolic::eq::{Expr, FixEq, FixType, VarId};

pub struct Node {
    pub id: usize,
    pub relevance: usize,
    pub player: Player,
    pub successors: Vec<usize>,
}

pub struct ParityGame {
    pub nodes: Vec<Node>,
}

pub fn parse_parity_game(source: &str) -> Result<ParityGame, Vec<Simple<char>>> {
    let parity = just("parity").padded();
    let number = text::int(10).map(|n: String| n.parse::<usize>().unwrap()).padded();
    let comma = just(',').padded();
    let semi = just(';');
    let newline = text::newline();

    let header = parity.ignore_then(number).then_ignore(semi).then_ignore(newline);

    let player = choice((just('0').to(Player::P0), just('1').to(Player::P1)));
    let successors = number.separated_by(comma);
    let comment = none_of(";");
    let row = number.then(number).then(player).then(successors).then_ignore(comment);
    let row = row.map(|(((id, relevance), player), successors)| Node {
        id,
        relevance,
        player,
        successors,
    });

    let rows = row.then_ignore(semi).separated_by(newline).allow_trailing();
    let game = header.ignore_then(rows).map(|nodes| ParityGame { nodes });

    // TODO: Validation
    game.parse(source)
}

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

    let funs_formulas = FunsFormulas::new(IndexedVec::new(), 1);

    (eqs, funs_formulas, node_id_to_var_id)
}

#[cfg(test)]
mod test {
    use crate::strategy::solve::solve;
    use crate::symbolic::compose::EqsFormulas;
    use crate::symbolic::formula::BasisElemId;

    use super::*;

    fn run_test(input: &str) {
        let game = parse_parity_game(input).unwrap();
        let (eqs, funs_formulas, node_id_to_var_id) = parity_game_to_fix(&game);
        let formulas = EqsFormulas::new(&eqs, &funs_formulas);

        let init_b = BasisElemId(0);
        let init_v = node_id_to_var_id[&0]; // TODO

        let is_winning = solve(init_b, init_v, formulas);

        println!("{is_winning}");

        todo!()
    }

    macro_rules! declare_test {
        ($($name:ident),* $(,)?) => {
            $(
                #[test]
                fn $name() {
                    let input = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/test/pg/", stringify!($name)));
                    run_test(input)
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
    }
}
