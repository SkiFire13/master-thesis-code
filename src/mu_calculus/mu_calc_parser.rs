use chumsky::error::Simple;
use chumsky::primitive::{choice, end, just};
use chumsky::recursive::recursive;
use chumsky::text::TextParser as _;
use chumsky::{text, Parser};

#[derive(Clone)]
pub struct Label(pub String);

#[derive(Clone)]
pub enum Act {
    True,
    Label(Label),
    NotLabel(Label),
}

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

/// A parser for the following grammar:
///
/// <Atom> ::= `tt' | `ff' | `(' <MuCalc> `)'
///         | <Id>
/// <ModalOp> ::= `<' <Label> `>' <Atom>
///         | `[' <Label> `]' <Atom>
///         | <Atom>
/// <Conjunction> ::= <Atom> (`&&' <Atom>)*
/// <Disjuction>  ::= <Conjunction> (`||' <Conjunction>)*
/// <Fix> ::= | `mu' <Id> `.' <Disjunction>
///          | `nu' <Id> `.' <Disjunction>
/// <MuCalc> ::= <Fix> | <Disjunction>
/// <Label> ::= `true' | ( provided label ) | `!` ( provided label )
/// <Id> ::= ( a C-style identifier )
///
pub fn parse_mu_calc<'a>(
    labels: impl Iterator<Item = &'a str>,
    source: &str,
) -> Result<MuCalc, Vec<Simple<char>>> {
    let expr = recursive(|expr| {
        let var = text::ident().map(Var).padded();

        let act_true = just("true").to(Act::True);
        let label = choice(labels.map(|l| just(l.to_string())).collect::<Vec<_>>()).map(Label);
        let act_label = label.clone().map(Act::Label);
        let act_not_label = just("!").padded().ignore_then(label.map(Act::NotLabel));
        let act = choice((act_true, act_label, act_not_label)).padded().boxed();

        let tt = text::keyword("tt").map(|_| MuCalc::And(Vec::new()));
        let ff = text::keyword("ff").map(|_| MuCalc::Or(Vec::new()));
        let group = expr.delimited_by(just('('), just(')'));
        let var_atom = var.map(MuCalc::Var);
        let atom = choice((tt, ff, group, var_atom)).padded().boxed();

        let diamond_act = act.clone().delimited_by(just('<'), just('>')).padded();
        let diamond = diamond_act.then(atom.clone()).map(|(l, e)| MuCalc::Diamond(l, Box::new(e)));
        let boxx_act = act.delimited_by(just('['), just(']')).padded();
        let boxx = boxx_act.then(atom.clone()).map(|(l, e)| MuCalc::Box(l, Box::new(e)));
        let modal = choice((diamond, boxx, atom)).boxed();

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
