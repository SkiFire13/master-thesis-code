use anyhow::Result;
use chumsky::error::Simple;
use chumsky::primitive::{choice, end, just, none_of};
use chumsky::recursive::recursive;
use chumsky::text::{self, keyword, TextParser};
use chumsky::Parser;

use crate::{Act, MuCalc, Var};

pub fn parse_mucalc<'a>(source: &str) -> Result<MuCalc, Vec<Simple<char>>> {
    let expr = recursive(|expr| {
        let var = text::ident().map(Var).padded();

        let act_true = just("true").to(Act::True);
        let label = none_of(">]").repeated().collect::<String>();
        let act_label = label.clone().map(Act::Label);
        let act_not_label = just("!").padded().ignore_then(label.map(Act::NotLabel));
        let act = choice((act_true, act_not_label, act_label)).padded().boxed();

        let tt = text::keyword("true").map(|_| MuCalc::And(Vec::new()));
        let ff = text::keyword("false").map(|_| MuCalc::Or(Vec::new()));
        let group = expr.delimited_by(just('('), just(')'));
        let var_atom = var.map(MuCalc::Var);
        let atom = choice((tt, ff, group, var_atom)).padded().boxed();

        let mod_pre = |l, r, f| act.clone().delimited_by(just(l), just(r)).map(move |l| (f, l));
        let diam = mod_pre('<', '>', MuCalc::Diamond as fn(_, _) -> _).boxed();
        let boxx = mod_pre('[', ']', MuCalc::Box as fn(_, _) -> _).boxed();
        let modal = choice((diam, boxx)).repeated().then(atom).foldr(|(f, l), e| f(l, Box::new(e)));

        let and = modal.boxed().separated_by(just("&&").padded()).map(unwrap_one_or(MuCalc::And));
        let or = and.boxed().separated_by(just("||").padded()).map(unwrap_one_or(MuCalc::Or));

        let dot = just('.').padded();
        let eta = |eta, f| keyword(eta).padded().then(var).then(dot).map(move |((_, v), _)| (f, v));
        let mu = eta("mu", MuCalc::Mu as fn(_, _) -> _).boxed();
        let nu = eta("nu", MuCalc::Nu as fn(_, _) -> _).boxed();
        let fix = choice((mu, nu)).repeated().then(or).foldr(|(f, v), e| f(v, Box::new(e)));

        fix
    });

    expr.then_ignore(end()).parse(source)
}

fn unwrap_one_or<T>(f: impl Fn(Vec<T>) -> T + Clone) -> impl Fn(Vec<T>) -> T + Clone {
    move |mut v| match v.len() {
        1 => v.pop().unwrap(),
        _ => f(v),
    }
}
