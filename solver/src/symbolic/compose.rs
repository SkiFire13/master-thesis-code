use std::rc::Rc;

use crate::index::{AsIndex, IndexedVec};
use crate::Map;

use super::eq::{Expr, FixEq, FixType, FunId, VarId};
use super::formula::{simplify_and, simplify_or, BasisElemId, Formula};

#[derive(Clone)]
pub struct FunsFormulas {
    generators: IndexedVec<FunId, Rc<dyn Fn(BasisElemId) -> Formula>>,
    cache: IndexedVec<FunId, Map<BasisElemId, Rc<Formula>>>,
}

impl FunsFormulas {
    pub fn new(formulas: IndexedVec<FunId, IndexedVec<BasisElemId, Formula>>) -> Self {
        let generators =
            formulas.iter().map(|_| Rc::new(|_| Formula::TRUE) as Rc<dyn Fn(_) -> _>).collect();
        let cache = formulas
            .into_iter()
            .map(|formulas| formulas.into_enumerate().map(|(i, f)| (i, Rc::new(f))).collect())
            .collect();

        Self { generators, cache }
    }

    pub fn with_generators(
        generators: IndexedVec<FunId, Rc<dyn Fn(BasisElemId) -> Formula>>,
    ) -> Self {
        let cache = generators.iter().map(|_| Map::default()).collect();
        Self { generators, cache }
    }

    pub fn get(&mut self, b: BasisElemId, f: FunId) -> Rc<Formula> {
        self.cache[f].entry(b).or_insert_with(|| Rc::new((self.generators[f])(b))).clone()
    }
}

#[derive(Clone)]
pub struct EqsFormulas {
    // /// 2D array with BasisElemId indexing columns and VarId indexing rows.
    // moves: IndexedVec<VarId, IndexedVec<BasisElemId, Formula>>,
    // /// Type of fixpoint for each equation.
    // eq_fix_types: IndexedVec<VarId, FixType>,
    eqs: IndexedVec<VarId, FixEq>,
    cache: IndexedVec<VarId, Map<BasisElemId, Formula>>,

    funs: FunsFormulas,
}

impl EqsFormulas {
    pub fn new(eqs: IndexedVec<VarId, FixEq>, funs: FunsFormulas) -> Self {
        let cache = eqs.iter().map(|_| Map::default()).collect();
        Self { eqs, cache, funs }
    }

    pub(super) fn get(&mut self, b: BasisElemId, i: VarId) -> &Formula {
        self.cache[i]
            .entry(b)
            .or_insert_with(|| compose_moves(&self.eqs[i].expr, b, &mut self.funs))
    }

    pub fn eq_fix_type(&self, i: VarId) -> FixType {
        self.eqs[i].fix_type
    }

    pub fn var_count(&self) -> usize {
        self.eqs.len()
    }
}

fn compose_moves(expr: &Expr, b: BasisElemId, moves: &mut FunsFormulas) -> Formula {
    match expr {
        Expr::Var(i) => Formula::Atom(b, *i),
        Expr::And(exprs) => simplify_and(exprs.iter().map(|e| compose_moves(e, b, moves))),
        Expr::Or(exprs) => simplify_or(exprs.iter().map(|e| compose_moves(e, b, moves))),
        Expr::Fun(fun, args) => subst(&*moves.get(b, *fun).clone(), args, moves),
    }
}

fn subst(formula: &Formula, args: &[Expr], moves: &mut FunsFormulas) -> Formula {
    match formula {
        Formula::Atom(b, i) => compose_moves(&args[i.to_usize()], *b, moves),
        Formula::And(fs) => simplify_and(fs.iter().map(|f| subst(f, args, moves))),
        Formula::Or(fs) => simplify_or(fs.iter().map(|f| subst(f, args, moves))),
    }
}
