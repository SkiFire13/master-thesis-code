use super::eq::{Expr, FixEq, FixType, VarId};

pub fn normalize_sys(eqs: &[FixEq]) -> (Vec<FixEq>, Vec<VarId>) {
    let mut new_eqs = Vec::new();
    let vars = eqs
        .iter()
        .map(|eq| normalize_expr(&eq.expr, eq.fix_type, &mut new_eqs))
        .collect::<Vec<_>>();
    (new_eqs, vars)
}

pub fn normalize_expr(expr: &Expr, fix_type: FixType, out: &mut Vec<FixEq>) -> VarId {
    let normalize_many = |exprs: &[Expr], out: &mut Vec<FixEq>| {
        exprs
            .iter()
            .map(|expr| Expr::Var(normalize_expr(expr, fix_type, out)))
            .collect()
    };

    match expr {
        Expr::Var(x) => *x,
        Expr::And(children) => {
            let var = VarId(out.len());
            out.push(FixEq { var, fix_type, expr: Expr::BOT });
            out[var.0].expr = Expr::And(normalize_many(children, out));
            var
        }
        Expr::Or(children) => {
            let var = VarId(out.len());
            out.push(FixEq { var, fix_type, expr: Expr::BOT });
            out[var.0].expr = Expr::Or(normalize_many(children, out));
            var
        }
        Expr::Fun(fun, args) => {
            let var = VarId(out.len());
            out.push(FixEq { var, fix_type, expr: Expr::And(Vec::new()) });
            out[var.0].expr = Expr::Fun(*fun, normalize_many(args, out));
            var
        }
    }
}
