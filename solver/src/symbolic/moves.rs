use std::rc::Rc;

use crate::retain::{simplify, Simplify};

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
        P0Moves::from_formula(formulas.get(self.b, self.i))
    }
}

impl P1Pos {
    pub fn moves(&self) -> P1Moves {
        P1Moves(self.moves.clone(), 0)
    }
}

impl P0Moves {
    fn from_formula(formula: &Formula) -> Self {
        let inner = FormulaIter::new(formula);
        let exhausted = inner.is_false();
        P0Moves { exhausted, inner }
    }

    pub fn simplify(&mut self, mut assumption: impl FnMut(P0Pos) -> Assumption) {
        match self.inner.simplify(false, &mut assumption) {
            (Assumption::Win, _) => self.inner = FormulaIter::And(Vec::new()),
            (Assumption::Lose, _) | (_, Status::Reset) => self.exhausted = true,
            _ => {}
        }
    }

    pub fn is_exhausted(&self) -> bool {
        self.exhausted
    }
}

impl P1Moves {
    pub fn is_exhausted(&self) -> bool {
        let P1Moves(moves, index) = self;
        *index >= moves.len()
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Assumption {
    Win,
    Lose,
    Unknown,
}

#[derive(Clone, Copy)]
enum Status {
    Still,
    Step,
    Reset,
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

    fn simplify(
        &mut self,
        mut reset: bool,
        assumption: &mut impl FnMut(P0Pos) -> Assumption,
    ) -> (Assumption, Status) {
        match self {
            FormulaIter::Atom(p) => (assumption(*p), Status::Still),
            FormulaIter::And(iters) => {
                let mut adv = reset.then(|| 0);

                let cleared = simplify(iters, |_, new_i, iter| {
                    let (assumption, status) = iter.simplify(reset, assumption);
                    match status {
                        Status::Still => {}
                        Status::Step => reset = true,
                        Status::Reset => adv = adv.or((!reset).then(|| new_i)),
                    }
                    reset |= adv.is_some();
                    match assumption {
                        Assumption::Win => Simplify::Remove,
                        Assumption::Lose => Simplify::Clear,
                        Assumption::Unknown => Simplify::Keep,
                    }
                });

                let assumption = match () {
                    _ if cleared => Assumption::Lose,
                    _ if iters.is_empty() => Assumption::Win,
                    _ => Assumption::Unknown,
                };

                let status =
                    match adv.map(|adv| iters[..adv].iter_mut().rev().any(|iter| iter.advance())) {
                        None => Status::Still,
                        Some(true) => Status::Step,
                        Some(false) => Status::Reset,
                    };

                if iters.len() == 1 {
                    *self = iters.pop().unwrap();
                }

                (assumption, status)
            }
            FormulaIter::Or(iters, pos) => {
                if reset {
                    *pos = 0;
                }

                let mut pos_data = (0, false, Status::Still);
                let mut winning = None;

                simplify(iters, |old_i, new_i, iter| {
                    let (assumption, status) = iter.simplify(reset, assumption);

                    if old_i == *pos {
                        pos_data = (new_i, assumption == Assumption::Lose, status);
                    }

                    if let Assumption::Win = assumption {
                        winning = winning.or(Some(old_i));
                    }

                    match assumption {
                        Assumption::Win => Simplify::Clear,
                        Assumption::Lose => Simplify::Remove,
                        Assumption::Unknown => Simplify::Keep,
                    }
                });

                let (new_pos, pos_lose, pos_status) = pos_data;

                match winning {
                    Some(wpos) if wpos < *pos => return (Assumption::Win, Status::Reset),
                    Some(wpos) if wpos > *pos => return (Assumption::Win, Status::Step),
                    Some(wpos) if wpos == *pos => return (Assumption::Win, pos_status),
                    _ => {}
                }

                if iters.is_empty() {
                    return (Assumption::Lose, Status::Still);
                }

                let (new_pos, status) = match pos_status {
                    _ if new_pos >= iters.len() => (0, Status::Reset),
                    _ if pos_lose => (new_pos, Status::Step),
                    Status::Reset if new_pos + 1 == iters.len() => (0, Status::Reset),
                    Status::Reset => (new_pos + 1, Status::Step),
                    Status::Step => (new_pos, Status::Step),
                    Status::Still => (new_pos, Status::Still),
                };
                *pos = new_pos;

                if iters.len() == 1 {
                    *self = iters.pop().unwrap();
                }

                (Assumption::Unknown, status)
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
    use itertools::Itertools;

    use super::{Assumption, BasisElemId, Formula, VarId};
    use crate::index::AsIndex;
    use crate::retain::Simplify;
    use crate::symbolic::moves::{P0Moves, P1Pos};

    macro_rules! formula {
        ($i:literal) => { Formula::Atom(BasisElemId($i), VarId($i)) };
        (( $($data:tt)+ )) => { formula!( $($data)* ) };
        ($($i:tt)&+) => { Formula::And(vec![ $( formula!($i) ),+ ]) };
        ($($i:tt)|+) => { Formula::Or(vec![ $( formula!($i) ),+ ]) };
    }

    fn all_moves(f: &Formula) -> Vec<Vec<usize>> {
        match f {
            // For tests we always have atoms with b = i
            Formula::Atom(b, _) => vec![vec![b.to_usize()]],
            Formula::And(children) => children
                .iter()
                .map(all_moves)
                .multi_cartesian_product()
                .map(|moves| moves.concat())
                .collect(),
            Formula::Or(children) => children.iter().flat_map(all_moves).collect(),
        }
    }

    fn filter_moves(moves: &mut Vec<Vec<usize>>, assumptions: impl Fn(usize) -> Assumption) {
        moves.retain_mut(|mov| {
            !crate::retain::simplify(mov, |_, _, pos| match assumptions(*pos) {
                Assumption::Win => Simplify::Remove,
                Assumption::Lose => Simplify::Clear,
                Assumption::Unknown => Simplify::Keep,
            })
        });

        let mut i = 0;
        'outer: while i < moves.len() {
            for mov in moves[..i].iter().chain(&moves[i + 1..]) {
                if mov.iter().all(|b| moves[i].contains(b)) {
                    moves.swap_remove(i);
                    continue 'outer;
                }
            }
            i += 1;
        }

        moves.iter_mut().for_each(|mov| mov.sort());
        moves.sort();
    }

    fn check_moves(f: &Formula, moves: &[P1Pos], assumptions: impl Fn(usize) -> Assumption) {
        let mut got_moves = moves
            .iter()
            .map(|pos| pos.moves.iter().map(|pos| pos.b.to_usize()).collect())
            .collect::<Vec<_>>();
        let mut all_moves = all_moves(f);

        filter_moves(&mut got_moves, &assumptions);
        filter_moves(&mut all_moves, &assumptions);

        if got_moves != all_moves {
            panic!("expected: {all_moves:?}\n     got: {got_moves:?}");
        }
    }

    macro_rules! test_formula {
        ($( $name:ident ( f = [$($f:tt)*], $($stmts:tt)* ) ),* $(,)?) => {
            $(
                #[allow(unused_mut)]
                #[test]
                fn $name() {
                    let f = formula!($($f)*);
                    let mut moves = P0Moves::from_formula(&f);
                    let mut out = Vec::new();

                    use std::collections::HashSet;
                    let mut winning = HashSet::<usize>::new();
                    let mut losing = HashSet::<usize>::new();

                    test_formula!(@STMT(moves out winning losing) $($stmts)*);

                    check_moves(&f, &out, |b| match () {
                        _ if winning.contains(&b) => Assumption::Win,
                        _ if losing.contains(&b) => Assumption::Lose,
                        _ => Assumption::Unknown,
                    });
                }
            )*
        };
        (@STMT($moves:ident $out:ident $winning:ident $losing:ident) next, $($stmts:tt)*) => {
            $out.push($moves.next().unwrap());
            test_formula!(@STMT($moves $out $winning $losing) $($stmts)*);
        };
        (@STMT($moves:ident $out:ident $winning:ident $losing:ident) simplify($($win:tt)*), $($stmts:tt)*) => {
            test_formula!(@WIN($winning $losing) $($win)*);
            $moves.simplify(|pos| match () {
                _ if $winning.contains(&pos.b.to_usize()) => Assumption::Win,
                _ if $losing.contains(&pos.b.to_usize()) => Assumption::Lose,
                _ => Assumption::Unknown,
            });
            test_formula!(@STMT($moves $out $winning $losing) $($stmts)*);
        };
        (@WIN($winning:ident $losing:ident) win $i:literal $(, $($rest:tt)*)?) => {
            $winning.insert($i);
            $( test_formula!(@WIN($winning $losing)) $($rest)* )?
        };
        (@WIN($winning:ident $losing:ident) lose $i:literal $(, $($rest:tt)*)?) => {
            $losing.insert($i);
            $( test_formula!(@WIN($winning $losing)) $($rest)* )?
        };
        (@STMT($moves:ident $out:ident $winning:ident $losing:ident) rest) => {
            $out.extend($moves);
        };
    }

    test_formula! {
        simple(
            f = [ 0 | ((1 | 2) & (3 | 4 | 5) & (6 | 7)) ],
            rest
        ),
        simplify_l1(
            f = [ (1 | 2) & (3 | 4) ],
            next,
            simplify(lose 1),
            rest
        ),
        simplify_l2(
            f = [ (1 | 2) & (3 | 4) ],
            next,
            simplify(lose 2),
            rest
        ),
        simplify_l3(
            f = [ (1 | 2) & (3 | 4) ],
            next,
            simplify(lose 3),
            rest
        ),
        simplify_l4(
            f = [ (1 | 2) & (3 | 4) ],
            next,
            simplify(lose 4),
            rest
        ),
        simplify_w1(
            f = [ (1 | 2) & (3 | 4) ],
            next,
            simplify(win 1),
            rest
        ),
        simplify_w2(
            f = [ (1 | 2) & (3 | 4) ],
            next,
            simplify(win 2),
            rest
        ),
        simplify_w3(
            f = [ (1 | 2) & (3 | 4) ],
            next,
            simplify(win 3),
            rest
        ),
        simplify_w4(
            f = [ (1 | 2) & (3 | 4) ],
            next,
            simplify(win 4),
            rest
        ),
        regression_1(
            f = [ (1 | 2) & (3 | 4) & (5 | 6) ],
            next,
            simplify(win 2),
            rest
        ),
        regression_2(
            f = [ (1 | 2) & (3 | 4) & (5 | 6) ],
            next,
            next,
            simplify(win 2),
            rest
        ),
    }
}
