use super::expr::{Expr, FixEq, FixType, VarId};

pub fn normalize_sys(eqs: &[FixEq]) -> (Vec<FixEq>, Vec<VarId>) {
    let mut new_eqs = Vec::new();
    let vars = eqs
        .iter()
        .map(|eq| normalize_expr(&eq.expr, eq.fix_type, &mut new_eqs))
        .collect::<Vec<_>>();
    (new_eqs, vars)
}

pub fn normalize_expr(expr: &Expr, fix_type: FixType, out: &mut Vec<FixEq>) -> VarId {
    match expr {
        Expr::Var(x) => *x,
        Expr::And(children) => {
            let var = VarId(out.len());
            out.push(FixEq { var, fix_type, expr: Expr::BOT });
            let new_children = children
                .iter()
                .map(|expr| Expr::Var(normalize_expr(expr, fix_type, out)))
                .collect();
            out[var.0].expr = Expr::And(new_children);
            var
        }
        Expr::Or(children) => {
            let var = VarId(out.len());
            out.push(FixEq { var, fix_type, expr: Expr::BOT });
            let new_children = children
                .iter()
                .map(|expr| Expr::Var(normalize_expr(expr, fix_type, out)))
                .collect();
            out[var.0].expr = Expr::Or(new_children);
            var
        }
        Expr::Fun(fun, args) => {
            let var = VarId(out.len());
            out.push(FixEq { var, fix_type, expr: Expr::And(Vec::new()) });
            let new_args = args
                .iter()
                .map(|expr| Expr::Var(normalize_expr(expr, fix_type, out)))
                .collect();
            out[var.0].expr = Expr::Fun(*fun, new_args);
            var
        }
    }
}
