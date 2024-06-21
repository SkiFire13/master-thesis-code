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
    pub fn simplify(&mut self, mut assumption: impl FnMut(P0Pos) -> Assumption) {
        match self.inner.simplify(&mut assumption) {
            Status::Winning => self.inner = FormulaIter::And(Vec::new()),
            Status::Losing => self.exhausted = true,
            Status::Advanced => {}
            Status::Exhausted => self.exhausted = true,
            Status::Still => {}
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
    Atom(P0Pos),
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

enum Status {
    // The subformula is definitely winning for P0.
    Winning,
    // The subformula is definitely losing for P0.
    Losing,
    // The current represented P1Pos was removed and replaced by a next one.
    Advanced,
    // All the remaining P1Pos were removed, but iterator can still be restarted.
    Exhausted,
    // The currently represented P1Pos was not removed.
    Still,
}

impl FormulaIter {
    fn new(f: &Formula) -> Self {
        match *f {
            Formula::Atom(b, i) => Self::Atom(P0Pos { b, i }),
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
            FormulaIter::Atom(_) => {}
            FormulaIter::And(iters) => iters.iter_mut().for_each(Self::reset),
            FormulaIter::Or(iters, pos) => {
                iters.iter_mut().for_each(Self::reset);
                *pos = 0;
            }
        }
    }

    fn simplify(&mut self, assumption: &mut impl FnMut(P0Pos) -> Assumption) -> Status {
        match self {
            FormulaIter::Atom(p) => match assumption(*p) {
                Assumption::Winning => Status::Winning,
                Assumption::Losing => Status::Losing,
                Assumption::Unknown => Status::Still,
            },
            FormulaIter::And(iters) => {
                let mut exhausted = None;
                let mut advanced = false;
                let mut curr_idx = 0;

                let new_iters = std::mem::take(iters)
                    .into_iter()
                    .simplify(|mut iter| {
                        if advanced || exhausted.is_some() {
                            iter.reset();
                        }

                        match iter.simplify(assumption) {
                            Status::Winning => return Simplify::Remove,
                            Status::Losing => return Simplify::Break,
                            Status::Advanced => advanced = true,
                            Status::Exhausted => exhausted = exhausted.or(Some(curr_idx)),
                            Status::Still => {}
                        }

                        curr_idx += 1;

                        Simplify::Keep(iter)
                    })
                    .collect::<Option<Vec<_>>>();

                let Some(new_iters) = new_iters else { return Status::Losing };
                *iters = new_iters;

                if iters.is_empty() {
                    // Formula is empty and thus is winning.
                    Status::Winning
                } else if iters.len() == 1 {
                    // Formula only contains one subformula and is thus equal to that one.
                    *self = iters.pop().unwrap();
                    // TODO: Is this correct?
                    match () {
                        _ if exhausted.is_some() => Status::Exhausted,
                        _ if advanced => Status::Advanced,
                        _ => Status::Still,
                    }
                } else if let Some(exhausted) = exhausted {
                    // One of the subformulas was exhausted, try advancing the earlier ones.
                    let advanced = iters[..exhausted].iter_mut().rev().any(|iter| iter.advance());
                    let status = if advanced { Status::Advanced } else { Status::Exhausted };
                    status
                } else {
                    // Nothing changed
                    Status::Still
                }
            }
            FormulaIter::Or(iters, pos) => {
                let mut new_pos = 0;
                let mut advanced = false;
                let mut exhausted = false;

                let new_iters = std::mem::take(iters)
                    .into_iter()
                    .enumerate()
                    .simplify(|(i, mut iter)| {
                        match iter.simplify(assumption) {
                            Status::Winning => return Simplify::Break,
                            Status::Losing => return Simplify::Remove,
                            Status::Advanced => advanced |= i == *pos,
                            Status::Exhausted => exhausted = true,
                            Status::Still => {}
                        }
                        new_pos += if i < *pos { 1 } else { 0 };
                        Simplify::Keep(iter)
                    })
                    .collect::<Option<Vec<_>>>();

                let Some(new_iters) = new_iters else { return Status::Winning };
                *iters = new_iters;
                *pos = new_pos;

                // Handle remaining with 0/1 subformulas
                if iters.is_empty() {
                    return Status::Losing;
                } else if iters.len() == 1 {
                    *self = iters.pop().unwrap();
                    return if exhausted { Status::Exhausted } else { Status::Advanced };
                }

                // Handle current formula being removed/advanced/exhausted.
                let (new_pos, status) = match () {
                    _ if *pos >= iters.len() => (0, Status::Exhausted),
                    _ if exhausted && *pos + 1 == iters.len() => (0, Status::Exhausted),
                    _ if exhausted && *pos + 1 < iters.len() => (*pos + 1, Status::Advanced),
                    _ if advanced => (*pos, Status::Advanced),
                    _ => (*pos, Status::Still),
                };
                *pos = new_pos;
                status
            }
        }
    }

    fn current(&self) -> Rc<[P0Pos]> {
        fn inner(iter: &FormulaIter, out: &mut Vec<P0Pos>) {
            match *iter {
                FormulaIter::Atom(p) => out.push(p),
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
            // An atom is always exhausted because it has only 1 item.
            FormulaIter::Atom(_) => false,
            // Try to advance any iterator from the last, just like adding 1 to a number.
            FormulaIter::And(iters) => iters.iter_mut().rev().any(|iter| iter.advance()),
            FormulaIter::Or(iters, pos) => {
                let (new_pos, advanced) = match () {
                    // Try to advance the current iterator
                    _ if iters[*pos].advance() => (*pos, true),
                    // Try to go to the next iterator
                    _ if *pos + 1 < iters.len() => (*pos + 1, true),
                    // We are exhausted ourself
                    _ => (0, false),
                };
                *pos = new_pos;
                advanced
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
