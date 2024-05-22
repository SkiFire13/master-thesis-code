use crate::index::{AsIndex, IndexVec};

use super::eq::{Expr, FixEq, FixType, FunId, VarId};
use super::formula::{simplify_and, simplify_or, BasisElemId, Formula};

pub struct FunsFormulas {
    formulas: IndexVec<FunId, IndexVec<BasisElemId, Formula>>,
}

impl FunsFormulas {
    pub fn get(&self, b: BasisElemId, f: FunId) -> &Formula {
        &self.formulas[f][b]
    }
}

pub struct EqsFormulas {
    // TODO: Make this an IndexVec<IndexVec>> ?
    /// 2D array with BasisElemId indexing columns and VarId indexing rows.
    pub moves: Vec<Formula>,
    /// Number of columns / length of a row.
    basis_len: usize,
    /// Type of fixpoint for each equation.
    pub eq_fix_types: IndexVec<VarId, FixType>,
}

impl EqsFormulas {
    pub fn new(eqs: &[FixEq], raw_moves: &FunsFormulas, basis_len: usize) -> Self {
        let mut moves = Vec::with_capacity(eqs.len() * raw_moves.formulas.len());
        for eq in eqs {
            for b in (0..basis_len).map(BasisElemId) {
                moves.push(compose_moves(&eq.expr, b, eqs, raw_moves));
            }
        }

        let eq_fix_types = eqs.iter().map(|e| e.fix_type).collect();

        Self { moves, basis_len, eq_fix_types }
    }

    pub fn get(&self, b: BasisElemId, i: VarId) -> &Formula {
        &self.moves[i.to_usize() * self.basis_len + b.to_usize()]
    }

    pub fn basis_len(&self) -> usize {
        self.basis_len
    }

    pub fn var_count(&self) -> usize {
        self.moves.len() / self.basis_len
    }
}

fn compose_moves(expr: &Expr, b: BasisElemId, eqs: &[FixEq], moves: &FunsFormulas) -> Formula {
    match expr {
        Expr::Var(i) => Formula::Atom(b, *i),
        Expr::And(exprs) => simplify_and(exprs.iter().map(|e| compose_moves(e, b, eqs, moves))),
        Expr::Or(exprs) => simplify_or(exprs.iter().map(|e| compose_moves(e, b, eqs, moves))),
        Expr::Fun(fun, args) => subst(moves.get(b, *fun), args, eqs, moves),
    }
}

fn subst(formula: &Formula, args: &[Expr], eqs: &[FixEq], moves: &FunsFormulas) -> Formula {
    match formula {
        Formula::Atom(b, i) => compose_moves(&args[i.to_usize()], *b, eqs, moves),
        Formula::And(fs) => simplify_and(fs.iter().map(|f| subst(f, args, eqs, moves))),
        Formula::Or(fs) => simplify_or(fs.iter().map(|f| subst(f, args, eqs, moves))),
    }
}
