use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufWriter, Write};

use rand::{thread_rng, Rng};

fn main() {
    let outpath = std::env::args().nth(1).expect("No output file provided");
    let nstates = std::env::args().nth(2).expect("No states count provided");
    let ntrans = std::env::args().nth(3).expect("No transitions count per vertex provided");
    let nlabels = std::env::args().nth(4).expect("No labels count provided");

    let nstates = nstates.parse::<usize>().expect("States count is not a valid number");
    let ntrans = ntrans.parse::<usize>().expect("Transitions count is not a valid number");
    let nlabels = nlabels.parse::<usize>().expect("Labels count is not a valid number");

    let out = File::create(outpath).expect("Cannot create output file");

    let mut rng = thread_rng();

    let mut lts = Vec::with_capacity(nstates);
    for _ in 0..nstates {
        let mut state_trans = BTreeSet::new();
        for _ in 0..ntrans {
            loop {
                let target = rng.gen_range(0..nstates);
                let label = rng.gen_range(0..nlabels);

                if state_trans.insert((target, label)) {
                    break;
                }
            }
        }
        lts.push(state_trans);
    }

    write_output(out, lts, ntrans).expect("Failed to write output file");
}

fn write_output(
    out: File,
    lts: Vec<BTreeSet<(usize, usize)>>,
    ntrans: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut out = BufWriter::new(out);

    writeln!(out, "des (0,{},{})", lts.len() * ntrans, lts.len())?;

    for (state, state_trans) in lts.into_iter().enumerate() {
        for (target, label) in state_trans {
            writeln!(out, "({},\"{}\",{})", state, label, target)?;
        }
    }

    Ok(())
}
