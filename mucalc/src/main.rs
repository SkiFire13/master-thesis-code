use std::rc::Rc;

use mucalc::{mucalc_to_fix, parse_alt, parse_mucalc};
use solver::local::solve;
use solver::symbolic::compose::EqsFormulas;

fn main() {
    let alt_path = std::env::args().nth(1).expect("No alt file provided");
    let mucalc_path = std::env::args().nth(2).expect("No mucalc file provided");

    let now = std::time::Instant::now();

    let alt_file = std::fs::read_to_string(alt_path).expect("Failed to read alt file");
    let mucalc_file = std::fs::read_to_string(mucalc_path).expect("Failed to read mucalc file");

    let lts = parse_alt(&alt_file).expect("Failed to parse alt file");
    let mucalc = parse_mucalc(&mucalc_file).expect("Failed to parse mucalc file");

    let (eqs, funs_formulas) = mucalc_to_fix(&mucalc, &lts);
    let init_b = lts.first_state.to_basis_elem();
    let init_v = eqs.last_index().unwrap();
    let formulas = Rc::new(EqsFormulas::new(eqs, Rc::new(funs_formulas)));

    println!("Preprocessing took {:?}", now.elapsed());

    let now = std::time::Instant::now();

    let is_winning = solve(init_b, init_v, formulas);

    println!("Solve took {:?}", now.elapsed());
    println!("The formula {} satisfied", if is_winning { "is" } else { "is not" });
}
