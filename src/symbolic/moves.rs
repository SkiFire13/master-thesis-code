use std::collections::HashSet;

use super::eq::VarId;
use super::formula::{BasisElemId, Formula};

impl Formula {
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
}
