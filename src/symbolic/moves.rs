use std::collections::HashSet;

use indexmap::IndexSet;

use crate::symbolic::formula::simplify_and;

use super::eq::VarId;
use super::formula::{simplify_or, BasisElemId, Formula};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Assumption {
    None,
    True,
    False,
}

impl Formula {
    pub fn with_assumptions(
        &self,
        assumptions: &impl Fn(BasisElemId, VarId) -> Assumption,
    ) -> Formula {
        match *self {
            Formula::Atom(b, i) => match assumptions(b, i) {
                Assumption::None => Formula::Atom(b, i),
                Assumption::True => Formula::TRUE,
                Assumption::False => Formula::FALSE,
            },
            Formula::And(ref children) => {
                simplify_and(children.iter().map(|f| f.with_assumptions(&assumptions)))
            }
            Formula::Or(ref children) => {
                simplify_or(children.iter().map(|f| f.with_assumptions(assumptions)))
            }
        }
    }

    pub fn apply_assumptions(
        self,
        assumptions: &impl Fn(BasisElemId, VarId) -> Assumption,
    ) -> Formula {
        match self {
            Formula::Atom(b, i) => match assumptions(b, i) {
                Assumption::None => Formula::Atom(b, i),
                Assumption::True => Formula::TRUE,
                Assumption::False => Formula::FALSE,
            },
            Formula::And(children) => simplify_and(
                children
                    .into_iter()
                    .map(|f| f.apply_assumptions(&assumptions)),
            ),
            Formula::Or(children) => simplify_or(
                children
                    .into_iter()
                    .map(|f| f.apply_assumptions(assumptions)),
            ),
        }
    }

    pub fn next_move(&self) -> Option<Vec<(BasisElemId, VarId)>> {
        match self {
            _ if self.is_false() => None,
            _ if self.is_true() => Some(Vec::new()),
            _ => Some(self.build_next_move()),
        }
    }

    fn build_next_move(&self) -> Vec<(BasisElemId, VarId)> {
        fn build_next_move_inner(f: &Formula, add: &mut impl FnMut(BasisElemId, VarId)) {
            match f {
                Formula::Atom(b, i) => add(*b, *i),
                Formula::And(children) => {
                    for f in children {
                        build_next_move_inner(f, add);
                    }
                }
                Formula::Or(children) => build_next_move_inner(&children[0], add),
            }
        }

        let mut out = Vec::new();
        let mut seen = HashSet::new();
        build_next_move_inner(self, &mut |b, i| {
            // Avoid pushing diplicates
            if seen.insert((b, i)) {
                out.push((b, i));
            }
        });

        // TODO: which is the best order?
        // Sorting because this needs to be normalized.
        out.sort_unstable_by_key(|&(b, i)| (i, b));

        out
    }

    pub fn next_move_optimized(
        &self,
        assumptions: impl Fn(BasisElemId, VarId) -> Assumption,
    ) -> Option<Vec<(BasisElemId, VarId)>> {
        fn build_next_move_inner(
            f: &Formula,
            assumptions: &impl Fn(BasisElemId, VarId) -> Assumption,
            out: &mut IndexSet<(BasisElemId, VarId)>,
        ) -> Assumption {
            match *f {
                Formula::Atom(b, i) => {
                    let assumption = assumptions(b, i);
                    if assumption == Assumption::None {
                        out.insert((b, i));
                    }
                    assumption
                }
                Formula::And(ref children) => {
                    // Loop over all children and add (b, i)s to make all true
                    let start = out.len();
                    for f in children {
                        // False subformula, the whole formula is false, rollback and return false.
                        if let Assumption::False = build_next_move_inner(f, assumptions, out) {
                            out.truncate(start);
                            return Assumption::False;
                        }
                    }
                    // If the length hasn't changed then all the subformulas were true.
                    if out.len() == start {
                        return Assumption::True;
                    }
                    Assumption::None
                }
                Formula::Or(ref children) => {
                    let mut children = children.iter();

                    // First loop: try finding a subformula that's neither true nor false.
                    let start = out.len();
                    for f in children.by_ref() {
                        match build_next_move_inner(f, assumptions, out) {
                            // We found one.
                            Assumption::None => break,
                            // False formula, ignore.
                            Assumption::False => {}
                            // True formula, rollback and propagate the assumption.
                            Assumption::True => {
                                out.truncate(start);
                                return Assumption::True;
                            }
                        }
                    }

                    // If the length is still the same it means we haven't found one, return false.
                    if out.len() == start {
                        return Assumption::False;
                    }

                    // Second loop: try finding a true formula.
                    // TODO: optimize to avoid having to rollback every time?
                    let last = out.len();
                    for f in children {
                        match build_next_move_inner(f, assumptions, out) {
                            // True, rollback everything and propagate the assumption.
                            Assumption::True => {
                                out.truncate(start);
                                return Assumption::True;
                            }
                            // Not true, rollback to last
                            _ => out.truncate(last),
                        }
                    }

                    Assumption::None
                }
            }
        }

        if self.is_false() {
            return None;
        }

        let mut out = IndexSet::new();
        match build_next_move_inner(self, &assumptions, &mut out) {
            Assumption::None => Some(out.into_iter().collect()),
            Assumption::True => Some(Vec::new()),
            Assumption::False => None,
        }
    }
}
