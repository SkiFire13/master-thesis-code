use std::collections::HashSet;
use std::rc::Rc;

use indexmap::IndexSet;

use crate::symbolic::formula::simplify_and;

use super::compose::EqsFormulas;
use super::eq::VarId;
use super::formula::{simplify_or, BasisElemId, Formula};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct P0Pos {
    pub b: BasisElemId,
    pub i: VarId,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct P1Pos {
    pub moves: Rc<[(BasisElemId, VarId)]>,
}

pub struct P0Moves(FormulaIter);

pub struct P1Moves(Rc<[(BasisElemId, VarId)]>, usize);

impl P0Pos {
    pub fn moves(&self, formulas: &EqsFormulas) -> P0Moves {
        P0Moves(FormulaIter::new(formulas.get(self.b, self.i)))
    }
}

impl P1Pos {
    pub fn moves(&self) -> P1Moves {
        P1Moves(self.moves.clone(), 0)
    }
}

impl Iterator for P0Moves {
    type Item = P1Pos;

    fn next(&mut self) -> Option<Self::Item> {
        Some(P1Pos { moves: self.0.next()? })
    }
}

impl Iterator for P1Moves {
    type Item = P0Pos;

    fn next(&mut self) -> Option<Self::Item> {
        let &(b, i) = self.0.get(self.1)?;
        self.1 += 1;
        Some(P0Pos { b, i })
    }
}

impl Default for P1Moves {
    fn default() -> Self {
        Self(Rc::new([]), 0)
    }
}

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
            Formula::And(children) => {
                simplify_and(children.into_iter().map(|f| f.apply_assumptions(&assumptions)))
            }
            Formula::Or(children) => {
                simplify_or(children.into_iter().map(|f| f.apply_assumptions(assumptions)))
            }
        }
    }

    pub fn next_move(&self) -> Option<Rc<[(BasisElemId, VarId)]>> {
        match self {
            _ if self.is_false() => None,
            _ if self.is_true() => Some(Rc::new([])),
            _ => Some(self.build_next_move().into()),
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
    ) -> Option<Rc<[(BasisElemId, VarId)]>> {
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
            Assumption::None => {
                let mut out = out.into_iter().collect::<Rc<[_]>>();
                // TODO: which is the best order?
                // Sorting because this needs to be normalized.
                Rc::get_mut(&mut out).unwrap().sort_unstable_by_key(|&(b, i)| (i, b));
                Some(out)
            }
            Assumption::True => Some(Rc::new([])),
            Assumption::False => self.next_move(),
        }
    }
}

struct FormulaIter {
    has_next: bool,
    inner: FormulaIterInner,
}

impl FormulaIter {
    fn new(f: &Formula) -> Self {
        fn new_inner(f: &Formula) -> FormulaIterInner {
            use FormulaIterInner::*;
            match *f {
                Formula::Atom(b, i) => Atom(b, i),
                Formula::And(ref children) => And(children.iter().map(new_inner).collect()),
                Formula::Or(ref children) => Or(children.iter().map(new_inner).collect(), 0),
            }
        }

        // We initially have a next value iff the formula is not false.
        let has_next = !f.is_false();
        let inner = new_inner(f);
        Self { has_next, inner }
    }

    // TODO: Need way to permanently apply assumption to a FormulaIter
}

impl Iterator for FormulaIter {
    type Item = Rc<[(BasisElemId, VarId)]>;

    fn next(&mut self) -> Option<Self::Item> {
        // Handle false formula and end of iterator.
        if !self.has_next {
            return None;
        }

        // The current value is the one to yield.
        let value = self.inner.current();
        // Try advancing, if we reset then we reached the end of the iterator.
        self.has_next = self.inner.advance();

        Some(value)
    }
}

enum FormulaIterInner {
    Atom(BasisElemId, VarId),
    // Contains iterators for subformulas.
    And(Vec<FormulaIterInner>),
    // Contains iterators for subformulas and the currently active subformula.
    Or(Vec<FormulaIterInner>, usize),
}

impl FormulaIterInner {
    fn current(&self) -> Rc<[(BasisElemId, VarId)]> {
        fn current_inner(iter: &FormulaIterInner, out: &mut impl FnMut(BasisElemId, VarId)) {
            match *iter {
                // An atom always yields itself
                FormulaIterInner::Atom(b, i) => _ = out(b, i),
                FormulaIterInner::And(ref iters) => {
                    // An And yield one item for each subformula
                    iters.iter().for_each(|iter| current_inner(iter, out))
                }
                // An or yields the item yielded by the currently active subformula.
                FormulaIterInner::Or(ref iters, pos) => current_inner(&iters[pos], out),
            }
        }

        // Deduplicate
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        current_inner(self, &mut |b, i| {
            if seen.insert((b, i)) {
                out.push((b, i))
            }
        });

        // TODO: which is the best order?
        // Sorting because this needs to be normalized.
        out.sort_unstable_by_key(|&(b, i)| (i, b));

        out.into()
    }

    // Advances to the next position or resets to the start if it reached the end.
    // Returns whether it reached the end.
    fn advance(&mut self) -> bool {
        match self {
            // An atom always resets because it has only 1 item.
            FormulaIterInner::Atom(_, _) => false,
            FormulaIterInner::And(iters) => {
                // Try to advance the last one, if it resets then advance the previous one
                // and so on. This is similar to how adding 1 to 199 turns into 200.
                for iter in iters.iter_mut().rev() {
                    if iter.advance() {
                        return true;
                    }
                }
                // If all sub-iterators resetted then we also reset.
                false
            }
            FormulaIterInner::Or(iters, pos) => {
                if iters[*pos].advance() {
                    // We successfully advanced the current iterator
                    true
                } else if *pos + 1 < iters.len() {
                    // Otherwise the current iterator resetted, go to the next one.
                    *pos += 1;
                    true
                } else {
                    // If there's no next iterator then we reset too.
                    *pos = 0;
                    false
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BasisElemId, Formula, FormulaIter, VarId};
    use std::rc::Rc;

    fn naive_moves(f: &Formula) -> Vec<Vec<(BasisElemId, VarId)>> {
        match *f {
            Formula::Atom(b, i) => vec![vec![(b, i)]],
            Formula::And(ref children) => {
                children.iter().map(naive_moves).fold(vec![Vec::new()], |ms1, ms2| {
                    ms1.iter()
                        .flat_map(|m1| ms2.iter().map(|m2| [&m1[..], &m2[..]].concat()))
                        .collect()
                })
            }
            Formula::Or(ref children) => children.iter().flat_map(naive_moves).collect(),
        }
    }

    #[test]
    fn simple() {
        let f = Formula::Or(vec![
            Formula::Atom(BasisElemId(0), VarId(0)),
            Formula::And(vec![
                Formula::Or(vec![
                    Formula::Atom(BasisElemId(1), VarId(1)),
                    Formula::Atom(BasisElemId(2), VarId(2)),
                ]),
                Formula::Or(vec![
                    Formula::Atom(BasisElemId(3), VarId(3)),
                    Formula::Atom(BasisElemId(4), VarId(4)),
                    Formula::Atom(BasisElemId(5), VarId(5)),
                ]),
                Formula::Or(vec![
                    Formula::Atom(BasisElemId(6), VarId(6)),
                    Formula::Atom(BasisElemId(7), VarId(7)),
                ]),
            ]),
        ]);

        let naive = naive_moves(&f)
            .into_iter()
            .map(|mut m| {
                m.sort_unstable_by_key(|&(b, i)| (i, b));
                m.dedup();
                Rc::<[_]>::from(m)
            })
            .collect::<Vec<_>>();

        let with_iter = FormulaIter::new(&f).collect::<Vec<_>>();

        assert_eq!(naive, with_iter);
    }
}
