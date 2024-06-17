use std::collections::HashMap;

use chumsky::error::Simple;
use chumsky::primitive::{choice, just, none_of};
use chumsky::text::{self, TextParser};
use chumsky::Parser;

use crate::index::IndexedVec;
use crate::strategy::game::Player;
use crate::symbolic::compose::FunsFormulas;
use crate::symbolic::eq::{Expr, FixEq, FixType, VarId};

#[derive(Debug)]
pub struct Node {
    pub id: usize,
    pub relevance: usize,
    pub player: Player,
    pub successors: Vec<usize>,
}

#[derive(Debug)]
pub struct ParityGame {
    pub nodes: Vec<Node>,
}

pub fn parse_parity_game(source: &str) -> Result<ParityGame, Vec<Simple<char>>> {
    let parity = just("parity").padded();
    let number = text::int(10).map(|n: String| n.parse::<usize>().unwrap()).padded();
    let comma = just(',').padded();
    let semi = just(';');
    let newline = text::newline();

    let header = parity.then(number).then(semi).then(newline);

    let player = choice((just('0').to(Player::P0), just('1').to(Player::P1)));
    let successors = number.separated_by(comma);
    let comment = none_of(";").repeated();
    let row = number.then(number).then(player).then(successors).then_ignore(comment);
    let row = row.map(|(((id, relevance), player), successors)| Node {
        id,
        relevance,
        player,
        successors,
    });

    let rows = row.then_ignore(semi).separated_by(newline).allow_trailing();
    let game = header.ignore_then(rows).map(|nodes| ParityGame { nodes });

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
    use std::rc::Rc;

    use crate::strategy::solve::solve;
    use crate::symbolic::compose::EqsFormulas;
    use crate::symbolic::formula::BasisElemId;

    use super::*;

    fn parse_parity_sol(source: &str) -> Result<Vec<(usize, Player)>, Vec<Simple<char>>> {
        let paritysol = just("paritysol").padded();
        let number = text::int(10).map(|n: String| n.parse::<usize>().unwrap()).padded();
        let semi = just(';');
        let newline = text::newline();

        let header = paritysol.then(number).then(semi).then(newline);

        let player = choice((just('0').to(Player::P0), just('1').to(Player::P1)));
        let succ = none_of(";").repeated();
        let row = number.then(player).then_ignore(succ);

        let rows = row.then_ignore(semi).separated_by(newline).allow_trailing();
        let sol = header.ignore_then(rows);

        sol.parse(source)
    }

    fn run_test(input: &str, sol: &str) {
        let game = parse_parity_game(input).unwrap();
        let (eqs, funs_formulas, node_id_to_var_id) = parity_game_to_fix(&game);
        let formulas = Rc::new(EqsFormulas::new(&eqs, &funs_formulas));
        let init_b = BasisElemId(0);

        let sol = parse_parity_sol(sol).unwrap();

        for (n, winner) in sol {
            let init_v = node_id_to_var_id[&n];

            let is_winning = solve(init_b, init_v, formulas.clone());
            let expected_winning = winner == Player::P0;

            assert_eq!(is_winning, expected_winning);
        }
    }

    macro_rules! declare_test {
        ($($name:ident),* $(,)?) => {
            $(
                #[test]
                fn $name() {
                    let input = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/test/pg/", stringify!($name)));
                    let sol = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/test/pg/", stringify!($name), ".sol"));
                    run_test(input, sol)
                }
            )*
        };
    }

    #[test]
    fn all() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/test/pg/");
        for e in std::fs::read_dir(dir).unwrap() {
            let e = e.unwrap();

            let name = e.file_name().into_string().unwrap();
            let path = e.path();
            if name == ".gitignore" || path.extension() == Some("sol".as_ref()) {
                continue;
            }

            let input = std::fs::read_to_string(&path).unwrap();
            let sol = std::fs::read_to_string(path.with_extension("sol")).unwrap();

            if let Err(e) = std::panic::catch_unwind(|| run_test(&input, &sol)) {
                eprintln!("Test {name} failed");
                std::panic::resume_unwind(e);
            }
        }
    }

    declare_test! {
        small,
        vb001,
        vb008,
        vb013,
        vb059,
        vb133,
    }
}
