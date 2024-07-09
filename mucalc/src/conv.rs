use std::collections::HashSet;
use std::rc::Rc;

use solver::index::{AsIndex, IndexedSet, IndexedVec};
use solver::symbolic::compose::FunsFormulas;
use solver::symbolic::eq::{Expr, FixEq, FixType, FunId, VarId};
use solver::symbolic::formula::{BasisElemId, Formula};

use crate::{Act, Lts, MuCalc, StateId, Var};

impl StateId {
    pub fn to_basis_elem(self) -> BasisElemId {
        BasisElemId(self.to_usize())
    }
}

pub fn mucalc_to_fix(mu_calc: &MuCalc, lts: Rc<Lts>) -> (IndexedVec<VarId, FixEq>, FunsFormulas) {
    let mut ctx = ConvContext {
        lts: &lts,
        funcs: IndexedSet::default(),
        vars: IndexedSet::default(),
        scope_vars: HashSet::new(),
        formulas: IndexedVec::new(),
        sys: IndexedVec::new(),
    };

    // First gather all variables, as they will be needed before
    // their defining appearence.
    ctx.gather_vars(mu_calc);

    // Then actually convert the expression
    let expr = ctx.conv(mu_calc);

    // If the resulting expr is a var then the outmost expression is a fixpoint
    // and we don't need another equation. Otherwise add one.
    // This is an exact equation so the fixpoint type doesn't matter.
    if !matches!(expr, Expr::Var(_)) {
        ctx.sys.push(FixEq { fix_type: FixType::Min, expr });
    }

    (ctx.sys, FunsFormulas::with_generators(ctx.formulas))
}

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
enum FunKind {
    Diamond,
    Box,
}

struct ConvContext<'a> {
    lts: &'a Rc<Lts>,

    // Maps each combination of box/diamond + act to a function id
    funcs: IndexedSet<FunId, (FunKind, &'a Act)>,
    // Maps each variable name to an id (corresponding to its equation)
    vars: IndexedSet<VarId, &'a Var>,
    // Keeps track of variables in scope to disallow variables outside their fixpoint.
    scope_vars: HashSet<VarId>,

    // Output uncomposed formulas
    formulas: IndexedVec<FunId, Rc<dyn Fn(BasisElemId) -> Formula>>,
    // Output fixpoint equations
    sys: IndexedVec<VarId, FixEq>,
}

impl<'a> ConvContext<'a> {
    fn gather_vars(&mut self, f: &'a MuCalc) {
        match f {
            MuCalc::Var(_) => {}
            MuCalc::Diamond(_, e) | MuCalc::Box(_, e) => self.gather_vars(e),
            MuCalc::And(es) | MuCalc::Or(es) => es.iter().for_each(|e| self.gather_vars(e)),
            MuCalc::Mu(x, e) | MuCalc::Nu(x, e) => {
                self.gather_vars(e);
                // Ensure the variable is inserted after the inner ones are gathered,
                // so that more external fixpoints are last and thus more relevant.
                self.vars.insert(x);
            }
        }
    }

    fn conv_modal(&mut self, fun_kind: FunKind, act: &'a Act, e: &'a MuCalc) -> Expr {
        let fun = match self.funcs.get_index_of(&(fun_kind, act)) {
            Some(fun) => fun,
            None => {
                self.funcs.insert((fun_kind, act));

                let act = act.clone();
                let label_matches = move |label: &str| match &act {
                    Act::True => true,
                    Act::Label(x) if x == label => true,
                    Act::NotLabel(x) if x != label => true,
                    _ => false,
                };

                let make_formula = move |edges: &Vec<(String, StateId)>| {
                    let formulas = edges
                        .iter()
                        .filter(|&(label, _)| label_matches(label))
                        .map(|(_, node)| Formula::Atom(node.to_basis_elem(), VarId(0)))
                        .collect();
                    match fun_kind {
                        FunKind::Diamond => Formula::Or(formulas),
                        FunKind::Box => Formula::And(formulas),
                    }
                };

                let lts = self.lts.clone();
                self.formulas
                    .push(Rc::new(move |b| make_formula(&lts.transitions[StateId(b.to_usize())])))
            }
        };

        Expr::Fun(fun, vec![self.conv(e)])
    }

    fn conv_fix(&mut self, fix_type: FixType, x: &'a Var, e: &'a MuCalc) -> Expr {
        let i = self.vars.index_of(x);

        let is_new = self.scope_vars.insert(i);
        assert!(is_new, "Variable {} declared twice", x.0);
        let expr = self.conv(e);
        self.scope_vars.remove(&i);

        Expr::Var(self.sys.push(FixEq { fix_type, expr }))
    }

    fn conv(&mut self, f: &'a MuCalc) -> Expr {
        match f {
            MuCalc::Var(x) => {
                let i = self
                    .vars
                    .get_index_of(&x)
                    .unwrap_or_else(|| panic!("Variable {} was not declared", x.0));
                assert!(self.scope_vars.contains(&i), "Variable {} not in scope", x.0);
                Expr::Var(i)
            }
            MuCalc::Diamond(a, e) => self.conv_modal(FunKind::Diamond, a, e),
            MuCalc::Box(a, e) => self.conv_modal(FunKind::Box, a, e),
            MuCalc::And(es) => Expr::And(es.iter().map(|e| self.conv(e)).collect()),
            MuCalc::Or(es) => Expr::Or(es.iter().map(|e| self.conv(e)).collect()),
            MuCalc::Mu(x, e) => self.conv_fix(FixType::Min, x, e),
            MuCalc::Nu(x, e) => self.conv_fix(FixType::Max, x, e),
        }
    }
}
