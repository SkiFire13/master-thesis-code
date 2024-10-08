use std::rc::Rc;

use aut::{parse_aut, StateId};
use bisimilarity::{bisimilarity_to_fix, make_basis_elem};
use solver::local::solve;
use solver::symbolic::compose::EqsFormulas;
use solver::symbolic::eq::VarId;

fn main() {
    let alt1_path = std::env::args().nth(1).expect("No first alt file provided");
    let alt2_path = std::env::args().nth(2).expect("No second alt file provided");

    let now = std::time::Instant::now();

    let alt1_file = std::fs::read_to_string(alt1_path).expect("Failed to read first alt file");
    let alt2_file = std::fs::read_to_string(alt2_path).expect("Failed to read second alt file");

    let lts1 = Rc::new(parse_aut(&alt1_file).expect("Failed to parse alt file"));
    let lts2 = Rc::new(parse_aut(&alt2_file).expect("Failed to parse alt file"));

    let parse_state = |s: &str| StateId(s.parse().expect("Failed to parse state id"));

    let init1 = std::env::args().nth(3).map(|s| parse_state(&s)).unwrap_or(lts1.first_state);
    let init2 = std::env::args().nth(4).map(|s| parse_state(&s)).unwrap_or(lts1.first_state);

    let (eqs, funs_formulas) = bisimilarity_to_fix(lts1.clone(), lts2.clone());

    let formulas = Rc::new(EqsFormulas::new(eqs, Rc::new(funs_formulas)));
    let init_b = make_basis_elem(init1, init2, &lts1, &lts2);
    let init_v = VarId(0);

    println!("Preprocessing took {:?}", now.elapsed());

    let now = std::time::Instant::now();

    let is_winning = solve(init_b, init_v, formulas);

    println!("Solve took {:?}", now.elapsed());
    println!("The formula {} satisfied", if is_winning { "is" } else { "is not" });
}
