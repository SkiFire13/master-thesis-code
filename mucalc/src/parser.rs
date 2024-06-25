use anyhow::{bail, Context, Result};
use chumsky::error::Simple;
use chumsky::primitive::{choice, end, just, none_of};
use chumsky::recursive::recursive;
use chumsky::text::{self, TextParser};
use chumsky::Parser;
use solver::index::IndexedVec;

use crate::{Act, Lts, MuCalc, StateId, Var};

pub fn parse_alt(source: &str) -> Result<Lts> {
    let mut lines = source.lines();

    let header = lines.next().context("File is empty")?;
    let header = header.strip_prefix("des").context("Expected 'des'")?;
    let header = header.trim().strip_prefix("(").context("Expected '('")?;
    let (first_state, header) = header.split_once(',').context("Expected first state")?;
    let (trans_count, header) = header.split_once(',').context("Expected trans count")?;
    let state_count = header.strip_suffix(")").context("Expected state count")?;

    let first_state = first_state.trim().parse().context("Expected first state to be a number")?;
    let trans_count = trans_count.trim().parse().context("Expected trans count to be a number")?;
    let state_count = state_count.trim().parse().context("Expected state count to be a number")?;

    if first_state >= state_count {
        bail!("First state {first_state} doesn't exist")
    }
    let first_state = StateId(first_state);

    let mut transitions = IndexedVec::from(vec![Vec::new(); state_count]);
    let mut transitions_count = 0usize;

    for line in lines {
        let line = line.strip_prefix('(').context("Expected '('")?;
        let (start_state, line) = line.split_once(',').context("Expected start state")?;
        let (label, line) = match line.trim_start().strip_prefix('"') {
            Some(line) => {
                let (label, line) = line.split_once('"').context("Expected label '\"'")?;
                let line = line.trim_start().strip_prefix(',').context("Expected label ','")?;
                (label, line)
            }
            None => line.split_once(',').context("Expected label")?,
        };
        let end_state = line.strip_suffix(')').context("Expected end state")?;

        let start_state = start_state.trim().parse().context("Start state is not a number")?;
        let end_state = end_state.trim().parse().context("End state is not a number")?;

        if start_state >= state_count {
            bail!("Start state {start_state} doesn't exist")
        }
        if end_state >= state_count {
            bail!("End state {end_state} doesn't exist")
        }

        transitions[StateId(start_state)].push((label.trim().to_string(), StateId(end_state)));
        transitions_count += 1;
    }

    if transitions_count != trans_count {
        bail!("Wrong number of transitions: got {transitions_count}, expected {trans_count}");
    }

    Ok(Lts { first_state, transitions })
}

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

        let modal = recursive(|modal| {
            let diam_act = act.clone().delimited_by(just('<'), just('>')).padded();
            let diam = diam_act.then(modal.clone()).map(|(l, e)| MuCalc::Diamond(l, Box::new(e)));
            let boxx_act = act.delimited_by(just('['), just(']')).padded();
            let boxx = boxx_act.then(modal.clone()).map(|(l, e)| MuCalc::Box(l, Box::new(e)));
            choice((diam, boxx, atom))
        });

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
