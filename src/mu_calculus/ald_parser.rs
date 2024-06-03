use chumsky::error::Simple;
use chumsky::primitive::{any, end, just};
use chumsky::text::{self, TextParser as _};
use chumsky::Parser;

use crate::index::{new_index, IndexedSet, IndexedVec};

new_index!(pub index StateId);
new_index!(pub index LabelId);

pub struct Lts {
    pub first_state: StateId,
    pub labels: IndexedSet<LabelId, String>,
    pub transitions: IndexedVec<StateId, Vec<(LabelId, StateId)>>,
}

// aut_header        ::=  'des (' first_state ',' nr_of_transitions ',' nr_of_states ')'
// first_state       ::=  number
// nr_of_transitions ::=  number
// nr_of_states      ::=  number
// aut_edge    ::=  '(' start_state ',' label ',' end_state ')'
// start_state ::=  number
// label       ::=  '"' string '"'
// end_state   ::=  number
pub fn parse_alt(source: &str) -> Result<Lts, Vec<Simple<char>>> {
    let des = just("des").padded();
    let number = text::int(10).map(|n: String| n.parse::<usize>().unwrap()).padded();
    let comma = just(',').padded();
    let newline = text::newline();
    let state = number.map(StateId);
    let label = any().repeated().collect::<String>().delimited_by(just('"'), just('"'));

    let inner = state.then_ignore(comma).then(number).then_ignore(comma).then(number);
    let header = des.ignore_then(inner.delimited_by(just('('), just(')'))).then_ignore(newline);

    let edge_inner = state.then_ignore(comma).then(label).then_ignore(comma).then(state);
    let edges = edge_inner.delimited_by(just('('), just(')')).padded().separated_by(newline);

    let parser = header.boxed().then_with(|((first_state, trans_count), states_count)| {
        edges.exactly(trans_count).map(move |edges| {
            let mut labels = IndexedSet::default();
            let mut transitions = IndexedVec::from(vec![Vec::new(); states_count]);

            for ((start_state, label), end_state) in edges {
                let (label_id, _) = labels.insert_full(label);
                transitions[start_state].push((label_id, end_state));
            }

            Lts { first_state, labels, transitions }
        })
    });

    parser.then_ignore(end()).parse(source)
}
