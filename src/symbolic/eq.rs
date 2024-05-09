use crate::index::new_index;

new_index!(pub index VarId);
new_index!(pub index FunId);

pub enum Expr {
    Var(VarId),
    And(Vec<Expr>),
    Or(Vec<Expr>),
    Fun(FunId, Vec<Expr>),
}

impl Expr {
    pub const BOT: Expr = Expr::Or(Vec::new());
    pub const TOP: Expr = Expr::And(Vec::new());
}

#[derive(Clone, Copy)]
pub enum FixType {
    Min,
    Max,
}

pub struct FixEq {
    pub var: VarId,
    pub fix_type: FixType,
    pub expr: Expr,
}
