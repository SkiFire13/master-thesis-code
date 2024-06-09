use std::collections::HashSet;

use chumsky::error::Simple;
use chumsky::primitive::{choice, end, just, none_of};
use chumsky::recursive::recursive;
use chumsky::text::TextParser as _;
use chumsky::{text, Parser};

use crate::index::{new_index, AsIndex, IndexedSet, IndexedVec};
use crate::symbolic::compose::FunsFormulas;
use crate::symbolic::eq::{Expr, FixEq, FixType, FunId, VarId};
use crate::symbolic::formula::{BasisElemId, Formula};

new_index!(pub index StateId);
new_index!(pub index LabelId);

impl StateId {
    pub fn to_basis_elem(self) -> BasisElemId {
        BasisElemId(self.to_usize())
    }
}

pub struct Lts {
    pub first_state: StateId,
    pub labels: IndexedSet<LabelId, String>,
    pub transitions: IndexedVec<StateId, Vec<(LabelId, StateId)>>,
}

// aut_header        ::=  'des (' first_state ',' nr_of_transitions ',' nr_of_states ')'
// first_state       ::=  number
// nr_of_transitions ::=  number
// nr_of_states      ::=  number
// aut_edge    ::=  '(' start_state ',' label ',' end_state ')'
// start_state ::=  number
// label       ::=  '"' string '"'
// end_state   ::=  number
pub fn parse_alt(source: &str) -> Result<Lts, Vec<Simple<char>>> {
    let des = just("des").padded();
    let number = text::int(10).map(|n: String| n.parse::<usize>().unwrap()).padded();
    let comma = just(',').padded();
    let newline = text::newline();
    let state = number.map(StateId);
    let label = none_of('"').repeated().collect::<String>().delimited_by(just('"'), just('"'));

    let inner = state.then_ignore(comma).then(number).then_ignore(comma).then(number);
    let header = des.ignore_then(inner.delimited_by(just('('), just(')'))).then_ignore(newline);

    let edge = state.then_ignore(comma).then(label).then_ignore(comma).then(state);
    let edges = edge.delimited_by(just('('), just(')')).separated_by(newline).allow_trailing();

    let parser = header.boxed().then_with(|((first_state, trans_count), states_count)| {
        edges.clone().exactly(trans_count).map(move |edges| {
            let mut labels = IndexedSet::default();
            let mut transitions = IndexedVec::from(vec![Vec::new(); states_count]);

            for ((start_state, label), end_state) in edges {
                let (label_id, _) = labels.insert_full(label);
                transitions[start_state].push((label_id, end_state));
            }

            Lts { first_state, labels, transitions }
        })
    });

    parser.then_ignore(end()).parse(source)
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum Act {
    True,
    Label(String),
    NotLabel(String),
}

#[derive(Hash, PartialEq, Eq)]
pub struct Var(pub String);

pub enum MuCalc {
    Var(Var),
    Diamond(Act, Box<MuCalc>),
    Box(Act, Box<MuCalc>),
    And(Vec<MuCalc>),
    Or(Vec<MuCalc>),
    Mu(Var, Box<MuCalc>),
    Nu(Var, Box<MuCalc>),
}

// <Atom> ::= `tt' | `ff' | `(' <MuCalc> `)'
//         | <Id>
// <ModalOp> ::= `<' <Label> `>' <Atom>
//         | `[' <Label> `]' <Atom>
//         | <Atom>
// <Conjunction> ::= <Atom> (`&&' <Atom>)*
// <Disjuction>  ::= <Conjunction> (`||' <Conjunction>)*
// <Fix> ::= | `mu' <Id> `.' <Disjunction>
//          | `nu' <Id> `.' <Disjunction>
// <MuCalc> ::= <Fix> | <Disjunction>
// <Label> ::= `true' | ( provided label ) | `!` ( provided label )
// <Id> ::= ( a C-style identifier )
pub fn parse_mu_calc<'a>(
    labels: impl Iterator<Item = &'a str>,
    source: &str,
) -> Result<MuCalc, Vec<Simple<char>>> {
    let expr = recursive(|expr| {
        let var = text::ident().map(Var).padded();

        let act_true = just("true").to(Act::True);
        let label = choice(labels.map(|l| just(l.to_string())).collect::<Vec<_>>());
        let act_label = label.clone().map(Act::Label);
        let act_not_label = just("!").padded().ignore_then(label.map(Act::NotLabel));
        let act = choice((act_true, act_label, act_not_label)).padded().boxed();

        let tt = text::keyword("tt").map(|_| MuCalc::And(Vec::new()));
        let ff = text::keyword("ff").map(|_| MuCalc::Or(Vec::new()));
        let group = expr.delimited_by(just('('), just(')'));
        let var_atom = var.map(MuCalc::Var);
        let atom = choice((tt, ff, group, var_atom)).padded().boxed();

        let diam_act = act.clone().delimited_by(just('<'), just('>')).padded();
        let diam = diam_act.then(atom.clone()).map(|(l, e)| MuCalc::Diamond(l, Box::new(e)));
        let boxx_act = act.delimited_by(just('['), just(']')).padded();
        let boxx = boxx_act.then(atom.clone()).map(|(l, e)| MuCalc::Box(l, Box::new(e)));
        let modal = choice((diam, boxx, atom)).boxed();

        let and = modal.separated_by(just("&&").padded()).map(MuCalc::And);
        let or = and.separated_by(just("||").padded()).map(MuCalc::Or);

        let dot = just('.').padded();
        let eta = |eta| text::keyword(eta).ignore_then(var).then_ignore(dot).then(or.clone());
        let mu = eta("mu").map(|(var, expr)| MuCalc::Mu(var, Box::new(expr)));
        let nu = eta("nu").map(|(var, expr)| MuCalc::Nu(var, Box::new(expr)));

        choice((mu, nu, or)).padded().boxed()
    });

    expr.then_ignore(end()).parse(source)
}

pub fn mu_calc_to_fix(mu_calc: &MuCalc, lts: &Lts) -> (IndexedVec<VarId, FixEq>, FunsFormulas) {
    match mu_calc {
        MuCalc::Mu(_, _) | MuCalc::Nu(_, _) => {}
        _ => panic!("mu-calculus formula must have a fix-point at the root"),
    }

    let mut ctx = ConvContext {
        lts,
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
    ctx.conv(mu_calc);

    (ctx.sys, FunsFormulas::new(ctx.formulas, lts.transitions.len()))
}

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
enum FunKind {
    Diamond,
    Box,
}

struct ConvContext<'a> {
    lts: &'a Lts,

    // Maps each combination of box/diamond + act to a function id
    funcs: IndexedSet<FunId, (FunKind, &'a Act)>,
    // Maps each variable name to an id (corresponding to its equation)
    vars: IndexedSet<VarId, &'a Var>,
    // Keeps track of variables in scope to disallow variables outside their fixpoint.
    scope_vars: HashSet<VarId>,

    // Output uncomposed formulas
    formulas: IndexedVec<FunId, IndexedVec<BasisElemId, Formula>>,
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
                let is_new = self.vars.insert(x);
                assert!(is_new, "Variable {} declared twice", x.0);
            }
        }
    }

    fn conv_modal(&mut self, fun_kind: FunKind, act: &'a Act, e: &'a MuCalc) -> Expr {
        let label_matches = |label| match act {
            Act::True => true,
            Act::Label(x) if x == &self.lts.labels[label] => true,
            Act::NotLabel(x) if x != &self.lts.labels[label] => true,
            _ => false,
        };

        let make_formula = |edges: &Vec<(LabelId, StateId)>| {
            let formulas = edges
                .iter()
                .filter(|&&(label, _)| label_matches(label))
                .map(|(_, node)| Formula::Atom(BasisElemId(node.to_usize()), VarId(0)))
                .collect();
            match fun_kind {
                FunKind::Diamond => Formula::Or(formulas),
                FunKind::Box => Formula::And(formulas),
            }
        };

        let fun = match self.funcs.get_index_of(&(fun_kind, act)) {
            Some(fun) => fun,
            None => {
                let fs = self.lts.transitions.iter().map(make_formula).collect::<Vec<_>>();
                self.funcs.insert((fun_kind, act));
                self.formulas.push(IndexedVec::from(fs))
            }
        };

        Expr::Fun(fun, vec![self.conv(e)])
    }

    fn conv_fix(&mut self, fix_type: FixType, x: &'a Var, e: &'a MuCalc) -> Expr {
        let i = self.vars.index_of(x);

        self.scope_vars.insert(i);
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

#[cfg(test)]
mod test {
    use crate::strategy::solve::solve;
    use crate::symbolic::compose::EqsFormulas;

    use super::*;

    #[test]
    fn gossips() {
        let aut_file = "./test/mucalc/gossips.aut";
        let mucalc_file = "./test/mucalc/gossips_known_after_7_steps";

        let alt_data = std::fs::read_to_string(aut_file).unwrap();
        let lts = parse_alt(&alt_data).unwrap();

        let mucalc_data = std::fs::read_to_string(mucalc_file).unwrap();
        let labels = lts.labels.iter().map(|s| &**s);
        let mucalc = parse_mu_calc(labels, &mucalc_data).unwrap();

        let (eqs, raw_formulas) = mu_calc_to_fix(&mucalc, &lts);

        let init_b = lts.first_state.to_basis_elem();
        let init_v = eqs.last_index().unwrap();
        let formulas = EqsFormulas::new(&eqs, &raw_formulas);

        let is_valid = solve(init_b, init_v, formulas);

        assert!(is_valid);
    }
}
