use std::rc::Rc;

use solver::local::solve;
use solver::symbolic::compose::EqsFormulas;

use crate::{mucalc_to_fix, parse_alt, parse_mucalc};

fn run_test(aut_path: &str, mucalc_path: &str, expected: bool) {
    let aut = std::fs::read_to_string(aut_path).unwrap();
    let lts = parse_alt(&aut).unwrap();

    let mucalc = std::fs::read_to_string(mucalc_path).unwrap();
    let parse_mu_calc = parse_mucalc(&mucalc);
    let mucalc = parse_mu_calc.unwrap();

    let (eqs, funs_formulas) = mucalc_to_fix(&mucalc, &lts);

    let init_b = lts.first_state.to_basis_elem();
    let init_v = eqs.last_index().unwrap();
    let formulas = Rc::new(EqsFormulas::new(&eqs, &funs_formulas));

    let is_valid = solve(init_b, init_v, formulas);

    assert_eq!(is_valid, expected);
}

macro_rules! declare_test {
        ($($aut:ident : [$($f:ident $(: $valid:literal)?),* $(,)?]),* $(,)?) => { $($(
            #[test]
            fn $f() {
                let aut = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/", stringify!($aut), ".aut");
                let mucalc = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/", stringify!($f));

                run_test(aut, mucalc, true $(&& $valid)?);
            }
        )*)* };
    }

declare_test! {
    bridge: [
        bridge_receive_17,
        bridge_report_17,
    ],
    gossips: [
        gossips_known_after_7_steps,
        gossips_known_after_7_steps_mu,
        gossips_deadlock_liveness,
    ],
    vm01: [
        vm01_a_always_eventually_ready,
        vm01_b_ready_always_possible: false,
        vm01_c_only_coin_after_ready,
        vm01_d_ready_coin_ready
    ],
    vm02: [
        vm02_a_no_3_10ct,
        vm02_b_no_chocolate_10,
    ],
}
