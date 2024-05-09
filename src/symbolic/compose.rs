use crate::index::IndexVec;

use super::eq::{Expr, FixEq, FixType, FunId, VarId};
use super::formula::{simplify_and, simplify_or, BasisId, Formula};

pub struct FunsFormulas {
    formulas: IndexVec<FunId, IndexVec<BasisId, Formula>>,
}

impl FunsFormulas {
    pub fn get(&self, basis: BasisId, fun: FunId) -> &Formula {
        &self.formulas[fun][basis]
    }
}

pub struct EqsFormulas {
    // TODO: Make this an IndexVec<IndexVec>> ?
    /// 2D array with BasisId indexing columns and VarId indexing rows.
    pub moves: Vec<Formula>,
    /// Number of columns / length of a row.
    basis_count: usize,
    /// Type of fixpoint for each equation.
    pub eq_fix_types: IndexVec<VarId, FixType>,
}

impl EqsFormulas {
    pub fn new(eqs: &[FixEq], raw_moves: &FunsFormulas, basis_count: usize) -> Self {
        let mut moves = Vec::with_capacity(eqs.len() * raw_moves.formulas.len());
        for eq in eqs {
            for b in (0..basis_count).map(BasisId) {
                moves.push(compose_moves(&eq.expr, b, eqs, raw_moves));
            }
        }

        let eq_fix_types = eqs.iter().map(|e| e.fix_type).collect();

        Self { moves, basis_count, eq_fix_types }
    }

    pub fn get(&self, b: BasisId, i: VarId) -> &Formula {
        &self.moves[i.0 * self.basis_count + b.0]
    }

    pub fn basis_count(&self) -> usize {
        self.basis_count
    }

    pub fn var_count(&self) -> usize {
        self.moves.len() / self.basis_count
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
