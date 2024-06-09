use fp_solver::symbolic::compose::EqsFormulas;

fn main() {
    let aut_file = "./test/mucalc/gossips.aut";
    let mucalc_file = "./test/mucalc/gossips_known_after_7_steps";

    let alt_data = std::fs::read_to_string(aut_file).unwrap();
    let lts = fp_solver::mu_calculus::parse_alt(&alt_data).unwrap();

    let mucalc_data = std::fs::read_to_string(mucalc_file).unwrap();
    let labels = lts.labels.iter().map(|s| &**s);
    let mucalc = fp_solver::mu_calculus::parse_mu_calc(labels, &mucalc_data).unwrap();

    let (eqs, raw_formulas) = fp_solver::mu_calculus::mu_calc_to_fix(&mucalc, &lts);

    let init_b = lts.first_state.to_basis_elem();
    let init_v = eqs.last_index().unwrap();
    let formulas = EqsFormulas::new(&eqs, &raw_formulas);

    let is_valid = fp_solver::strategy::solve::solve(init_b, init_v, formulas);

    println!("{}", is_valid);
}
