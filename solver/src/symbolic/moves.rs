use std::rc::Rc;

use crate::iter::{IteratorExt, Simplify};

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
    pub moves: Rc<[P0Pos]>,
}

pub struct P0Moves {
    exhausted: bool,
    inner: FormulaIter,
}

pub struct P1Moves(Rc<[P0Pos]>, usize);

impl P0Pos {
    pub fn moves(&self, formulas: &EqsFormulas) -> P0Moves {
        let inner = FormulaIter::new(formulas.get(self.b, self.i));
        let exhausted = inner.is_false();
        P0Moves { exhausted, inner }
    }
}

impl P1Pos {
    pub fn moves(&self) -> P1Moves {
        P1Moves(self.moves.clone(), 0)
    }
}

impl P0Moves {
    pub fn simplify(&mut self, mut assumption: impl FnMut(BasisElemId, VarId) -> Assumption) {
        match self.inner.simplify(&mut assumption) {
            Simplified::Winning => self.inner = FormulaIter::And(Vec::new()),
            Simplified::Losing => self.exhausted = true,
            Simplified::Updated => {}
            Simplified::Exhausted => self.exhausted = true,
            Simplified::Still => {}
        }
    }
}

impl Iterator for P0Moves {
    type Item = P1Pos;

    fn next(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }

        let moves = self.inner.current();

        if !self.inner.advance() {
            self.exhausted = true;
        }

        Some(P1Pos { moves })
    }
}

impl Iterator for P1Moves {
    type Item = P0Pos;

    fn next(&mut self) -> Option<Self::Item> {
        let P1Moves(moves, index) = self;
        let &pos = moves.get(*index)?;
        *index += 1;
        Some(pos)
    }
}

impl Default for P1Moves {
    fn default() -> Self {
        Self(Rc::new([]), 0)
    }
}

enum FormulaIter {
    Atom(BasisElemId, VarId),
    // Contains iterators for subformulas.
    And(Vec<FormulaIter>),
    // Contains iterators for subformulas and the currently active subformula.
    Or(Vec<FormulaIter>, usize),
}

pub enum Assumption {
    Winning,
    Losing,
    Unknown,
}

enum Simplified {
    // The subformula is definitely winning for P0.
    Winning,
    // The subformula is definitely losing for P0.
    Losing,
    // The current represented P1Pos was removed and replaced by a next one.
    Updated,
    // All the remaining P1Pos were removed, but iterator can still be restarted.
    Exhausted,
    // The currently represented P1Pos was not removed.
    Still,
}

impl FormulaIter {
    fn new(f: &Formula) -> Self {
        match *f {
            Formula::Atom(b, i) => Self::Atom(b, i),
            Formula::And(ref children) => Self::And(children.iter().map(Self::new).collect()),
            Formula::Or(ref children) => Self::Or(children.iter().map(Self::new).collect(), 0),
        }
    }

    fn is_false(&self) -> bool {
        let FormulaIter::Or(iters, _) = self else { return false };
        iters.is_empty()
    }

    fn reset(&mut self) {
        match self {
            FormulaIter::Atom(_, _) => {}
            FormulaIter::And(iters) => iters.iter_mut().for_each(Self::reset),
            FormulaIter::Or(iters, pos) => {
                iters.iter_mut().for_each(Self::reset);
                *pos = 0;
            }
        }
    }

    fn simplify(
        &mut self,
        assumption: &mut impl FnMut(BasisElemId, VarId) -> Assumption,
    ) -> Simplified {
        match self {
            FormulaIter::Atom(b, i) => match assumption(*b, *i) {
                Assumption::Winning => Simplified::Winning,
                Assumption::Losing => Simplified::Losing,
                Assumption::Unknown => Simplified::Still,
            },
            FormulaIter::And(iters) => {
                let mut advance_from = None;
                let mut need_reset = false;
                let mut curr_idx = 0;

                let new_iters = std::mem::take(iters)
                    .into_iter()
                    .simplify(|mut iter| {
                        if need_reset {
                            iter.reset();
                        }

                        match iter.simplify(assumption) {
                            Simplified::Winning => return Simplify::Remove,
                            Simplified::Losing => return Simplify::Break,
                            Simplified::Updated => need_reset = true,
                            Simplified::Exhausted => advance_from = Some(curr_idx),
                            Simplified::Still => {}
                        }

                        curr_idx += 1;

                        Simplify::Keep(iter)
                    })
                    .collect::<Option<Vec<_>>>();

                let Some(new_iters) = new_iters else { return Simplified::Losing };
                *iters = new_iters;

                if iters.is_empty() {
                    return Simplified::Winning;
                }

                if let Some(advance_from) = advance_from {
                    for iter in iters[..advance_from].iter_mut().rev() {
                        if iter.advance() {
                            return Simplified::Updated;
                        }
                    }

                    return Simplified::Exhausted;
                }

                Simplified::Still
            }
            FormulaIter::Or(iters, pos) => {
                let mut shift = 0;
                let mut exhausted = false;
                let mut updated = false;

                let new_iters = std::mem::take(iters)
                    .into_iter()
                    .enumerate()
                    .simplify_with(
                        |i| shift += (i < *pos) as usize,
                        |(i, mut iter)| {
                            match iter.simplify(assumption) {
                                Simplified::Winning => return Simplify::Break,
                                Simplified::Losing => return Simplify::Remove,
                                Simplified::Updated => updated |= i == *pos,
                                Simplified::Exhausted => exhausted = true,
                                Simplified::Still => {}
                            }
                            Simplify::Keep(iter)
                        },
                    )
                    .collect::<Option<Vec<_>>>();

                let Some(new_iters) = new_iters else { return Simplified::Winning };
                *iters = new_iters;
                *pos -= shift;

                if iters.is_empty() {
                    return Simplified::Losing;
                }

                if *pos >= iters.len() {
                    *pos = 0;
                    return Simplified::Exhausted;
                }

                if exhausted {
                    if *pos + 1 < iters.len() {
                        *pos += 1;
                        return Simplified::Updated;
                    } else {
                        *pos = 0;
                        return Simplified::Exhausted;
                    }
                }

                if updated {
                    return Simplified::Updated;
                }

                Simplified::Still
            }
        }
    }

    fn current(&self) -> Rc<[P0Pos]> {
        fn inner(iter: &FormulaIter, out: &mut Vec<P0Pos>) {
            match *iter {
                FormulaIter::Atom(b, i) => out.push(P0Pos { b, i }),
                FormulaIter::And(ref iters) => iters.iter().for_each(|iter| inner(iter, out)),
                FormulaIter::Or(ref iters, pos) => inner(&iters[pos], out),
            }
        }

        let mut out = Vec::new();
        inner(self, &mut out);

        // TODO: which is the best order?
        // Sorting because this needs to be normalized.
        out.sort_unstable_by_key(|&P0Pos { b, i }| (i, b));
        out.dedup();

        out.into()
    }

    // Advances to the next position or resets to the start if it reached the end.
    // Returns whether it reached the end.
    fn advance(&mut self) -> bool {
        match self {
            // An atom always resets because it has only 1 item.
            FormulaIter::Atom(_, _) => false,
            FormulaIter::And(iters) => {
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
            FormulaIter::Or(iters, pos) => {
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
    use super::{BasisElemId, Formula, P0Pos, VarId};
    use crate::symbolic::moves::{FormulaIter, P0Moves, P1Pos};

    fn naive_moves(f: &Formula) -> Vec<Vec<P0Pos>> {
        match *f {
            Formula::Atom(b, i) => vec![vec![P0Pos { b, i }]],
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
                m.sort_unstable_by_key(|&P0Pos { b, i }| (i, b));
                m.dedup();
                P1Pos { moves: m.into() }
            })
            .collect::<Vec<_>>();

        let formula_iter = FormulaIter::new(&f);
        let moves = P0Moves { exhausted: false, inner: formula_iter };
        let with_iter = moves.collect::<Vec<_>>();

        assert_eq!(naive, with_iter);
    }
}
