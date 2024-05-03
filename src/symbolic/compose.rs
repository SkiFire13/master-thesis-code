use super::eq::{Expr, FixEq, FunId, VarId};
use super::formula::{simplify_and, simplify_or, BasisId, Formula};

pub struct FunsFormulas {
    formulas: Vec<Vec<Formula>>,
}

impl FunsFormulas {
    pub fn get(&self, basis: BasisId, fun: FunId) -> &Formula {
        &self.formulas[fun.0][basis.0]
    }
}

pub struct EqsFormulas {
    /// 2D array with BasisId indexing columns and VarId indexing rows.
    moves: Vec<Formula>,
    /// Number of columns / length of a row.
    basis_count: usize,
}

impl EqsFormulas {
    pub fn compose(eqs: &[FixEq], raw_moves: &FunsFormulas, basis_count: usize) -> Self {
        let mut moves = Vec::with_capacity(eqs.len() * raw_moves.formulas.len());
        for eq in eqs {
            for b in (0..basis_count).map(BasisId) {
                moves.push(compose_moves(&eq.expr, b, eqs, raw_moves));
            }
        }

        Self { moves, basis_count }
    }

    pub fn get(&self, b: BasisId, i: VarId) -> &Formula {
        &self.moves[i.0 * self.basis_count + b.0]
    }
}

fn compose_moves(expr: &Expr, b: BasisId, eqs: &[FixEq], moves: &FunsFormulas) -> Formula {
    match expr {
        Expr::Var(i) => Formula::Atom(b, *i),
        Expr::And(exprs) => simplify_and(exprs.iter().map(|e| compose_moves(e, b, eqs, moves))),
        Expr::Or(exprs) => simplify_or(exprs.iter().map(|e| compose_moves(e, b, eqs, moves))),
        Expr::Fun(fun, args) => subst(moves.get(b, *fun), args, eqs, moves),
    }
}

fn subst(formula: &Formula, args: &[Expr], eqs: &[FixEq], moves: &FunsFormulas) -> Formula {
    match formula {
        Formula::Atom(b, i) => compose_moves(&args[i.0], *b, eqs, moves),
        Formula::And(fs) => simplify_and(fs.iter().map(|f| subst(f, args, eqs, moves))),
        Formula::Or(fs) => simplify_or(fs.iter().map(|f| subst(f, args, eqs, moves))),
    }
}
