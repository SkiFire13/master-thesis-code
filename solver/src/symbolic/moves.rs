use std::rc::Rc;

use super::compose::EqsFormulas;
use super::eq::VarId;
use super::formula::{BasisElemId, Formula};

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
        let P1Moves(moves, index) = self;
        let &(b, i) = moves.get(*index)?;
        *index += 1;
        Some(P0Pos { b, i })
    }
}

impl Default for P1Moves {
    fn default() -> Self {
        Self(Rc::new([]), 0)
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
        fn inner(iter: &FormulaIterInner, out: &mut Vec<(BasisElemId, VarId)>) {
            match *iter {
                FormulaIterInner::Atom(b, i) => out.push((b, i)),
                FormulaIterInner::And(ref iters) => iters.iter().for_each(|iter| inner(iter, out)),
                FormulaIterInner::Or(ref iters, pos) => inner(&iters[pos], out),
            }
        }

        let mut out = Vec::new();
        inner(self, &mut out);

        // TODO: which is the best order?
        // Sorting because this needs to be normalized.
        out.sort_unstable_by_key(|&(b, i)| (i, b));
        out.dedup();

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
