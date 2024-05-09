use crate::index::new_index;

use super::eq::VarId;

new_index!(pub index BasisId);

pub enum Formula {
    Atom(BasisId, VarId),
    And(Vec<Formula>),
    Or(Vec<Formula>),
}

impl Formula {
    pub const FALSE: Self = Formula::Or(Vec::new());
    pub const TRUE: Self = Formula::And(Vec::new());

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

    pub fn simplify(self) -> Self {
        match self {
            Formula::Atom(b, i) => Formula::Atom(b, i),
            Formula::And(children) => simplify_and(children.into_iter().map(Self::simplify)),
            Formula::Or(children) => simplify_or(children.into_iter().map(Self::simplify)),
        }
    }
}

pub fn simplify_and(iter: impl Iterator<Item = Formula>) -> Formula {
    let children = iter
        .filter(|f| !f.is_true())
        .map(|f| (!f.is_false()).then_some(f))
        .collect::<Option<Vec<_>>>();
    match children {
        None => Formula::FALSE,
        Some(children) if children.len() == 1 => children.into_iter().next().unwrap(),
        Some(children) => Formula::And(children),
    }
}

pub fn simplify_or(iter: impl Iterator<Item = Formula>) -> Formula {
    let children = iter
        .filter(|f| !f.is_false())
        .map(|f| (!f.is_true()).then_some(f))
        .collect::<Option<Vec<_>>>();
    match children {
        None => Formula::TRUE,
        Some(children) if children.len() == 1 => children.into_iter().next().unwrap(),
        Some(children) => Formula::Or(children),
    }
}
