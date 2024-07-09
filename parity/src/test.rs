use chumsky::error::Simple;
use chumsky::primitive::{choice, just, none_of};
use chumsky::text::{self, TextParser};
use chumsky::Parser;
use solver::local::solve;
use solver::strategy::Player;
use solver::symbolic::compose::EqsFormulas;
use solver::symbolic::formula::BasisElemId;

use crate::{parity_game_to_fix, parse_parity_game};

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
    let formulas = EqsFormulas::new(eqs, funs_formulas);
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
                let input = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/", stringify!($name)));
                let sol = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/", stringify!($name), ".sol"));
                run_test(input, sol)
            }
        )*
    };
}

#[test]
fn all() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/");
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
