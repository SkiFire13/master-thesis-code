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

impl P0Moves {
    pub fn simplify(&mut self, mut assumption: impl FnMut(BasisElemId, VarId) -> Assumption) {
        match self.0.inner.simplify(&mut assumption) {
            Assumption::Winning => self.0.inner = FormulaIterInner::And(Vec::new()),
            Assumption::Losing => self.0.inner = FormulaIterInner::Or(Vec::new(), 0),
            Assumption::Unknown => {}
        }
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
    is_first: bool,
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

        let inner = new_inner(f);
        Self { is_first: true, inner }
    }
}

impl Iterator for FormulaIter {
    type Item = Rc<[(BasisElemId, VarId)]>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_first {
            self.is_first = false;

            return match self.inner.is_false() {
                true => None,
                false => Some(self.inner.current()),
            };
        }

        if !self.inner.advance() {
            return None;
        }

        Some(self.inner.current())
    }
}

enum FormulaIterInner {
    Atom(BasisElemId, VarId),
    // Contains iterators for subformulas.
    And(Vec<FormulaIterInner>),
    // Contains iterators for subformulas and the currently active subformula.
    Or(Vec<FormulaIterInner>, usize),
}

pub enum Assumption {
    Winning,
    Losing,
    Unknown,
}

impl FormulaIterInner {
    fn is_false(&self) -> bool {
        let FormulaIterInner::Or(iters, _) = self else { return false };
        iters.is_empty()
    }

    fn simplify(
        &mut self,
        assumption: &mut impl FnMut(BasisElemId, VarId) -> Assumption,
    ) -> Assumption {
        match self {
            FormulaIterInner::Atom(b, i) => assumption(*b, *i),
            FormulaIterInner::And(iters) => {
                let new_iters = std::mem::take(iters)
                    .into_iter()
                    .filter_map(|mut f| match f.simplify(assumption) {
                        // Ignore the winning ones
                        Assumption::Winning => None,
                        // Short circuit if one is losing
                        Assumption::Losing => Some(None),
                        // Passthrough the unknowns
                        Assumption::Unknown => Some(Some(f)),
                    })
                    .collect::<Option<Vec<_>>>();

                let Some(new_iters) = new_iters else { return Assumption::Losing };
                *iters = new_iters;

                match iters.is_empty() {
                    true => Assumption::Winning,
                    false => Assumption::Unknown,
                }
            }
            FormulaIterInner::Or(iters, pos) => {
                let mut shift = 0;
                let new_iters = std::mem::take(iters)
                    .into_iter()
                    .map(|mut f| match f.simplify(assumption) {
                        // Ignore the losing ones
                        Assumption::Losing => None,
                        // Short circuit if one is winning
                        Assumption::Winning => Some(None),
                        // Passthrough the unknowns
                        Assumption::Unknown => Some(Some(f)),
                    })
                    .enumerate()
                    .inspect(|(i, o)| shift += (i < pos && o.is_none()) as usize)
                    .filter_map(|(_, o)| o)
                    .collect::<Option<Vec<_>>>();

                *pos -= shift;

                let Some(new_iters) = new_iters else { return Assumption::Winning };
                *iters = new_iters;

                match iters.is_empty() {
                    true => Assumption::Losing,
                    false => Assumption::Unknown,
                }
            }
        }
    }

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
                if *pos >= iters.len() {
                    // We removed all trailing iterators in a simplify
                    *pos = 0;
                    false
                } else if iters[*pos].advance() {
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
