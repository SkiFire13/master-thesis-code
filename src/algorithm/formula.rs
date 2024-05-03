use super::expr::{Expr, FixEq, FunId, VarId};

#[derive(Clone, Copy)]
pub struct BasisId(usize);

pub enum Formula {
    BasisElem(BasisId, VarId),
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
        Expr::Var(i) => Formula::BasisElem(b, *i),
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
        Formula::BasisElem(b, i) => compose_moves(&args[i.0], *b, eqs, moves),
        Formula::And(children) => Formula::And(subst_many(children)),
        Formula::Or(children) => Formula::Or(subst_many(children)),
    }
}

fn simplify(formula: Formula) -> Formula {
    match formula {
        Formula::BasisElem(_, _) => formula,
        Formula::And(children) => {
            let children = children
                .into_iter()
                .map(simplify)
                .filter(|f| !f.is_true())
                .map(|f| (!f.is_false()).then_some(f))
                .collect::<Option<Vec<_>>>();
            match children {
                None => Formula::FALSE,
                Some(children) if children.len() == 1 => children.into_iter().next().unwrap(),
                Some(children) => Formula::Or(children),
            }
        }
        Formula::Or(children) => {
            let children = children
                .into_iter()
                .map(simplify)
                .filter(|f| !f.is_false())
                .map(|f| (!f.is_true()).then_some(f))
                .collect::<Option<Vec<_>>>();
            match children {
                None => Formula::TRUE,
                Some(children) if children.len() == 1 => children.into_iter().next().unwrap(),
                Some(children) => Formula::Or(children),
            }
        }
    }
}
