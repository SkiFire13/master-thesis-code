use std::rc::Rc;

use parity::{parity_game_to_fix, parse_parity_game};
use solver::local::solve;
use solver::symbolic::compose::EqsFormulas;
use solver::symbolic::formula::BasisElemId;

fn main() {
    let path = std::env::args().nth(1).expect("No parity game file provided");
    let node = std::env::args()
        .nth(2)
        .map(|n| n.parse().expect("Failed to parse starting node"))
        .unwrap_or(0);

    let now = std::time::Instant::now();

    let file = std::fs::read_to_string(path).expect("Failed to read parity game file");
    let graph = parse_parity_game(&file).expect("Failed to parse parity game file");

    let (eqs, funs_formulas, node_id_to_var_id) = parity_game_to_fix(&graph);
    let formulas = Rc::new(EqsFormulas::new(&eqs, &funs_formulas));
    let init_b = BasisElemId(0);
    let init_v = node_id_to_var_id[&node];

    println!("Preprocessing took {:?}", now.elapsed());

    let now = std::time::Instant::now();

    let is_winning = solve(init_b, init_v, formulas);

    println!("Solve took {:?}", now.elapsed());
    println!("Winner: player {}", if is_winning { 0 } else { 1 });
}
