use std::collections::HashSet;

use super::expr::{Expr, FixEq, FunId, VarId};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BasisId(usize);

pub enum Formula {
    Atom(BasisId, VarId),
    And(Vec<Formula>),
    Or(Vec<Formula>),
}

impl Formula {
    pub const FALSE: Formula = Formula::Or(Vec::new());
    pub const TRUE: Formula = Formula::And(Vec::new());

    pub fn is_true(&self) -> bool {
        match self {
            Self::And(c) if c.is_empty() => true,
            _ => false,
        }
    }

    pub fn is_false(&self) -> bool {
        match self {
            Self::Or(c) if c.is_empty() => true,
            _ => false,
        }
    }

    pub fn next_move(&self) -> Option<Vec<(BasisId, VarId)>> {
        match self {
            _ if self.is_false() => None,
            _ if self.is_true() => Some(Vec::new()),
            _ => Some(self.build_next_move()),
        }
    }

    fn build_next_move(&self) -> Vec<(BasisId, VarId)> {
        fn build_next_move_inner(f: &Formula, add: &mut impl FnMut(BasisId, VarId)) {
            match f {
                Formula::Atom(b, i) => add(*b, *i),
                Formula::And(children) => {
                    for f in children {
                        build_next_move_inner(f, add);
                    }
                }
                Formula::Or(children) => build_next_move_inner(&children[0], add),
            }
        }

        let mut out = Vec::new();
        let mut seen = HashSet::new();
        build_next_move_inner(self, &mut |b, i| {
            if seen.insert((b, i)) {
                out.push((b, i));
            }
        });

        // TODO: is it better to sort? If yes, which is the best order?
        // out.sort_unstable_by_key(|&(b, i)| (i, b));
        out
    }
}

pub struct FunsFormulas {
    /// 2D array with BasisId indexing columns and FunId indexing rows.
    formulas: Vec<Formula>,
    /// Number of columns / length of a row.
    basis_count: usize,
}

impl FunsFormulas {
    pub fn get(&self, basis: BasisId, fun: FunId) -> &Formula {
        &self.formulas[fun.0 * self.basis_count + basis.0]
    }
}

pub struct EqsFormulas {
    /// 2D array with BasisId indexing columns and VarId indexing rows.
    moves: Vec<Formula>,
    /// Number of columns / length of a row.
    basis_count: usize,
}

impl EqsFormulas {
    pub fn compose(eqs: &[FixEq], raw_moves: &FunsFormulas) -> Self {
        let mut moves = Vec::with_capacity(eqs.len() * raw_moves.formulas.len());
        for eq in eqs {
            for b in 0..raw_moves.basis_count {
                // TODO: Merge simplify and compose_moves/subst together
                let f = simplify(compose_moves(&eq.expr, BasisId(b), eqs, raw_moves));
                moves.push(f);
            }
        }

        Self { moves, basis_count: raw_moves.basis_count }
    }

    pub fn get(&self, b: BasisId, i: VarId) -> &Formula {
        &self.moves[i.0 * self.basis_count + b.0]
    }
}

fn compose_moves(expr: &Expr, b: BasisId, eqs: &[FixEq], moves: &FunsFormulas) -> Formula {
    let compose_many = |exprs: &[Expr]| {
        exprs
            .iter()
            .map(|expr| compose_moves(expr, b, eqs, moves))
            .collect()
    };

    match expr {
        Expr::Var(i) => Formula::Atom(b, *i),
        Expr::And(children) => Formula::And(compose_many(children)),
        Expr::Or(children) => Formula::Or(compose_many(children)),
        Expr::Fun(fun, args) => subst(moves.get(b, *fun), args, eqs, moves),
    }
}

fn subst(formula: &Formula, args: &[Expr], eqs: &[FixEq], moves: &FunsFormulas) -> Formula {
    let subst_many = |formulas: &[Formula]| {
        formulas
            .iter()
            .map(|formula| subst(formula, args, eqs, moves))
            .collect()
    };

    match formula {
        Formula::Atom(b, i) => compose_moves(&args[i.0], *b, eqs, moves),
        Formula::And(children) => Formula::And(subst_many(children)),
        Formula::Or(children) => Formula::Or(subst_many(children)),
    }
}

fn simplify(formula: Formula) -> Formula {
    match formula {
        Formula::Atom(_, _) => formula,
        Formula::And(children) => simplify_and(children.into_iter().map(simplify)),
        Formula::Or(children) => simplify_or(children.into_iter().map(simplify)),
    }
}

fn simplify_and(iter: impl Iterator<Item = Formula>) -> Formula {
    let children = iter
        .filter(|f| !f.is_true())
        .map(|f| (!f.is_false()).then_some(f))
        .collect::<Option<Vec<_>>>();
    match children {
        None => Formula::FALSE,
        Some(children) if children.len() == 1 => children.into_iter().next().unwrap(),
        Some(children) => Formula::And(children),
    }
}

fn simplify_or(iter: impl Iterator<Item = Formula>) -> Formula {
    let children = iter
        .filter(|f| !f.is_false())
        .map(|f| (!f.is_true()).then_some(f))
        .collect::<Option<Vec<_>>>();
    match children {
        None => Formula::TRUE,
        Some(children) if children.len() == 1 => children.into_iter().next().unwrap(),
        Some(children) => Formula::Or(children),
    }
}

#[allow(unused)]
mod compose_simplify_merged {
    use super::*;

    fn compose_moves(expr: &Expr, b: BasisId, eqs: &[FixEq], moves: &FunsFormulas) -> Formula {
        match expr {
            Expr::Var(i) => Formula::Atom(b, *i),
            Expr::And(children) => simplify_and(
                children
                    .iter()
                    .map(|expr| compose_moves(expr, b, eqs, moves)),
            ),
            Expr::Or(children) => simplify_or(
                children
                    .iter()
                    .map(|expr| compose_moves(expr, b, eqs, moves)),
            ),
            Expr::Fun(fun, args) => subst(moves.get(b, *fun), args, eqs, moves),
        }
    }

    fn subst(formula: &Formula, args: &[Expr], eqs: &[FixEq], moves: &FunsFormulas) -> Formula {
        match formula {
            Formula::Atom(b, i) => compose_moves(&args[i.0], *b, eqs, moves),
            Formula::And(children) => simplify_and(
                children
                    .iter()
                    .map(|formula| subst(formula, args, eqs, moves)),
            ),
            Formula::Or(children) => simplify_or(
                children
                    .iter()
                    .map(|formula| subst(formula, args, eqs, moves)),
            ),
        }
    }
}
