use fp_solver::{
    self,
    index::AsIndex,
    symbolic::{compose::EqsFormulas, eq::VarId, formula::BasisElemId},
};

fn main() {
    let aut_file = "./test/mucalc/gossips.aut";
    let mucalc_file = "./test/mucalc/gossips_known_after_7_steps";

    let alt_data = std::fs::read_to_string(aut_file).unwrap();
    let mucalc_data = std::fs::read_to_string(mucalc_file).unwrap();

    let lts = fp_solver::mu_calculus::parse_alt(&alt_data).unwrap();
    let mucalc =
        fp_solver::mu_calculus::parse_mu_calc(lts.labels.iter().map(|s| &**s), &mucalc_data)
            .unwrap();

    let (eqs, fn_fs) = fp_solver::mu_calculus::mu_calc_to_fix(&mucalc, &lts);

    let valid = fp_solver::strategy::solve::solve(
        BasisElemId(lts.first_state.to_usize()),
        VarId(eqs.len() - 1),
        EqsFormulas::new(&eqs, &fn_fs, lts.transitions.len()),
    );

    println!("{}", valid);
}
