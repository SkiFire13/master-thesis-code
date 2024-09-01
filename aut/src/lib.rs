use anyhow::{bail, Context, Result};
use solver::index::{AsIndex as _, IndexedVec};
use solver::new_index;
use solver::symbolic::formula::BasisElemId;

new_index!(pub index StateId);

impl StateId {
    pub fn to_basis_elem(self) -> BasisElemId {
        BasisElemId(self.to_usize())
    }
}

pub struct Lts {
    pub first_state: StateId,
    pub transitions: IndexedVec<StateId, Vec<(String, StateId)>>,
}

pub fn parse_aut(source: &str) -> Result<Lts> {
    let mut lines = source.lines();

    let header = lines.next().context("File is empty")?;
    let header = header.strip_prefix("des").context("Expected 'des'")?;
    let header = header.trim().strip_prefix("(").context("Expected '('")?;
    let (first_state, header) = header.split_once(',').context("Expected first state")?;
    let (trans_count, header) = header.split_once(',').context("Expected trans count")?;
    let state_count = header.strip_suffix(")").context("Expected state count")?;

    let first_state = first_state.trim().parse().context("Expected first state to be a number")?;
    let trans_count = trans_count.trim().parse().context("Expected trans count to be a number")?;
    let state_count = state_count.trim().parse().context("Expected state count to be a number")?;

    if first_state >= state_count {
        bail!("First state {first_state} doesn't exist")
    }
    let first_state = StateId(first_state);

    let mut transitions = IndexedVec::from(vec![Vec::new(); state_count]);
    let mut transitions_count = 0usize;

    for line in lines {
        let line = line.strip_prefix('(').context("Expected '('")?;
        let (start_state, line) = line.split_once(',').context("Expected start state")?;
        let (label, line) = match line.trim_start().strip_prefix('"') {
            Some(line) => {
                let (label, line) = line.split_once('"').context("Expected label '\"'")?;
                let line = line.trim_start().strip_prefix(',').context("Expected label ','")?;
                (label, line)
            }
            None => line.split_once(',').context("Expected label")?,
        };
        let end_state = line.strip_suffix(')').context("Expected end state")?;

        let start_state = start_state.trim().parse().context("Start state is not a number")?;
        let end_state = end_state.trim().parse().context("End state is not a number")?;

        if start_state >= state_count {
            bail!("Start state {start_state} doesn't exist")
        }
        if end_state >= state_count {
            bail!("End state {end_state} doesn't exist")
        }

        transitions[StateId(start_state)].push((label.trim().to_string(), StateId(end_state)));
        transitions_count += 1;
    }

    if transitions_count != trans_count {
        bail!("Wrong number of transitions: got {transitions_count}, expected {trans_count}");
    }

    Ok(Lts { first_state, transitions })
}
