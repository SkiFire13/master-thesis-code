use solver::index::{IndexedSet, IndexedVec};
use solver::new_index;

mod conv;
mod parser;

#[cfg(test)]
mod test;

pub use conv::mucalc_to_fix;
pub use parser::{parse_alt, parse_mucalc};

new_index!(pub index StateId);
new_index!(pub index LabelId);

pub struct Lts {
    pub first_state: StateId,
    pub labels: IndexedSet<LabelId, String>,
    pub transitions: IndexedVec<StateId, Vec<(LabelId, StateId)>>,
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum Act {
    True,
    Label(String),
    NotLabel(String),
}

#[derive(Hash, PartialEq, Eq)]
pub struct Var(pub String);

pub enum MuCalc {
    Var(Var),
    Diamond(Act, Box<MuCalc>),
    Box(Act, Box<MuCalc>),
    And(Vec<MuCalc>),
    Or(Vec<MuCalc>),
    Mu(Var, Box<MuCalc>),
    Nu(Var, Box<MuCalc>),
}
