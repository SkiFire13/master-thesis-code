use crate::index::{AsIndex, IndexedVec};

use super::eq::{Expr, FixEq, FixType, FunId, VarId};
use super::formula::{simplify_and, simplify_or, BasisElemId, Formula};

pub struct FunsFormulas {
    formulas: IndexedVec<FunId, IndexedVec<BasisElemId, Formula>>,
    basis_len: usize,
}

impl FunsFormulas {
    pub fn new(
        formulas: IndexedVec<FunId, IndexedVec<BasisElemId, Formula>>,
        basis_len: usize,
    ) -> Self {
        Self { formulas, basis_len }
    }

    pub fn get(&self, b: BasisElemId, f: FunId) -> &Formula {
        &self.formulas[f][b]
    }
}

pub struct EqsFormulas {
    /// 2D array with BasisElemId indexing columns and VarId indexing rows.
    pub moves: IndexedVec<VarId, IndexedVec<BasisElemId, Formula>>,
    /// Type of fixpoint for each equation.
    pub eq_fix_types: IndexedVec<VarId, FixType>,
}

impl EqsFormulas {
    pub fn new(eqs: &[FixEq], raw_moves: &FunsFormulas) -> Self {
        let basis_len = raw_moves.basis_len;

        let moves = eqs
            .iter()
            .map(|eq| {
                (0..basis_len).map(|b| compose_moves(&eq.expr, BasisElemId(b), raw_moves)).collect()
            })
            .collect();

        let eq_fix_types = eqs.iter().map(|e| e.fix_type).collect();

        Self { moves, eq_fix_types }
    }

    pub fn get(&self, b: BasisElemId, i: VarId) -> &Formula {
        &self.moves[i][b]
    }

    pub fn var_count(&self) -> usize {
        self.moves.len()
    }
}

fn compose_moves(expr: &Expr, b: BasisElemId, moves: &FunsFormulas) -> Formula {
    match expr {
        Expr::Var(i) => Formula::Atom(b, *i),
        Expr::And(exprs) => simplify_and(exprs.iter().map(|e| compose_moves(e, b, moves))),
        Expr::Or(exprs) => simplify_or(exprs.iter().map(|e| compose_moves(e, b, moves))),
        Expr::Fun(fun, args) => subst(moves.get(b, *fun), args, moves),
    }
}

fn subst(formula: &Formula, args: &[Expr], moves: &FunsFormulas) -> Formula {
    match formula {
        Formula::Atom(b, i) => compose_moves(&args[i.to_usize()], *b, moves),
        Formula::And(fs) => simplify_and(fs.iter().map(|f| subst(f, args, moves))),
        Formula::Or(fs) => simplify_or(fs.iter().map(|f| subst(f, args, moves))),
    }
}
