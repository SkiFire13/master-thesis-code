mod conv;
mod parser;

#[cfg(test)]
mod test;

pub use aut::{parse_aut, Lts, StateId};
pub use conv::mucalc_to_fix;
pub use parser::parse_mucalc;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Act {
    True,
    Label(String),
    NotLabel(String),
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Var(pub String);

#[derive(Debug)]
pub enum MuCalc {
    Var(Var),
    Diamond(Act, Box<MuCalc>),
    Box(Act, Box<MuCalc>),
    And(Vec<MuCalc>),
    Or(Vec<MuCalc>),
    Mu(Var, Box<MuCalc>),
    Nu(Var, Box<MuCalc>),
}
