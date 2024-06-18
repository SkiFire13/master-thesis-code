use chumsky::error::Simple;
use chumsky::primitive::{choice, end, just, none_of};
use chumsky::recursive::recursive;
use chumsky::text::{self, TextParser};
use chumsky::Parser;
use solver::index::IndexedVec;

use crate::{Act, Lts, MuCalc, StateId, Var};

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
            let mut transitions = IndexedVec::from(vec![Vec::new(); states_count]);

            for ((start_state, label), end_state) in edges {
                transitions[start_state].push((label, end_state));
            }

            Lts { first_state, transitions }
        })
    });

    parser.then_ignore(end()).parse(source)
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
pub fn parse_mucalc<'a>(source: &str) -> Result<MuCalc, Vec<Simple<char>>> {
    let expr = recursive(|expr| {
        let var = text::ident().map(Var).padded();

        let act_true = just("true").to(Act::True);
        let label = none_of(">").repeated().collect::<String>();
        let act_label = label.clone().map(Act::Label);
        let act_not_label = just("!").padded().ignore_then(label.map(Act::NotLabel));
        let act = choice((act_true, act_not_label, act_label)).padded().boxed();

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
