use mucalc::{Lts, StateId};
use solver::index::{AsIndex, IndexedVec};
use solver::symbolic::compose::FunsFormulas;
use solver::symbolic::eq::{Expr, FixEq, FixType, FunId, VarId};
use solver::symbolic::formula::{BasisElemId, Formula};

pub fn bisimilarity_to_fix(lts1: &Lts, lts2: &Lts) -> (IndexedVec<VarId, FixEq>, FunsFormulas) {
    let eq = FixEq { fix_type: FixType::Max, expr: Expr::Fun(FunId(0), vec![Expr::Var(VarId(0))]) };
    let eqs = IndexedVec::from(vec![eq]);

    let formulas = lts1
        .transitions
        .indexes()
        .flat_map(|s1| lts2.transitions.indexes().map(move |s2| formula_for(s1, s2, lts1, lts2)))
        .collect();
    let funs_formulas = FunsFormulas::new(IndexedVec::from(vec![formulas]));

    (eqs, funs_formulas)
}

pub fn make_basis_elem(s1: StateId, s2: StateId, _lts1: &Lts, lts2: &Lts) -> BasisElemId {
    let (s1, s2) = (s1.to_usize(), s2.to_usize());
    BasisElemId(s1 * lts2.transitions.len() + s2)
}

fn formula_for(s1: StateId, s2: StateId, lts1: &Lts, lts2: &Lts) -> Formula {
    let left = lts1.transitions[s1].iter().map(|&(ref l1, n1)| {
        lts2.transitions[s2]
            .iter()
            .filter(|(l2, _)| l1 == l2)
            .map(|&(_, n2)| Formula::Atom(make_basis_elem(n1, n2, lts1, lts2), VarId(0)))
            .collect()
    });

    let right = lts2.transitions[s2].iter().map(|&(ref l2, n2)| {
        lts1.transitions[s1]
            .iter()
            .filter(|(l1, _)| l1 == l2)
            .map(|&(_, n1)| Formula::Atom(make_basis_elem(n1, n2, lts1, lts2), VarId(0)))
            .collect()
    });

    Formula::And(left.chain(right).map(Formula::Or).collect())
}
