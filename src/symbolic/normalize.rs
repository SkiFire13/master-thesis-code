use crate::index::IndexedVec;

use super::eq::{Expr, FixEq, FixType, VarId};

pub fn normalize_sys(eqs: &[FixEq]) -> (IndexedVec<VarId, FixEq>, IndexedVec<VarId, VarId>) {
    let mut new_eqs = IndexedVec::new();
    let vars = eqs.iter().map(|eq| normalize_expr(&eq.expr, eq.fix_type, &mut new_eqs)).collect();
    (new_eqs, vars)
}

pub fn normalize_expr(expr: &Expr, fix_type: FixType, out: &mut IndexedVec<VarId, FixEq>) -> VarId {
    let normalize_many = |exprs: &[Expr], out: &mut IndexedVec<VarId, FixEq>| {
        exprs.iter().map(|expr| Expr::Var(normalize_expr(expr, fix_type, out))).collect()
    };

    match expr {
        Expr::Var(x) => *x,
        Expr::And(children) => {
            // TODO: No var in Eq and let var = out.push(...)
            let var = VarId(out.len());
            out.push(FixEq { var, fix_type, expr: Expr::BOT });
            out[var].expr = Expr::And(normalize_many(children, out));
            var
        }
        Expr::Or(children) => {
            let var = VarId(out.len());
            out.push(FixEq { var, fix_type, expr: Expr::BOT });
            out[var].expr = Expr::Or(normalize_many(children, out));
            var
        }
        Expr::Fun(fun, args) => {
            let var = VarId(out.len());
            out.push(FixEq { var, fix_type, expr: Expr::And(Vec::new()) });
            out[var].expr = Expr::Fun(*fun, normalize_many(args, out));
            var
        }
    }
}
