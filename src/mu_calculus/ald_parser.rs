use chumsky::error::Simple;
use chumsky::primitive::{any, end, just};
use chumsky::text::{self, TextParser as _};
use chumsky::Parser;

#[derive(Clone)]
pub struct State(pub usize);

#[derive(Clone)]
pub struct Label(pub String);

pub struct Lts {
    pub first_state: State,
    pub transitions: Vec<Vec<(Label, State)>>,
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
    let state = number.map(State);
    let label = any().repeated().collect().delimited_by(just('"'), just('"')).map(Label);

    let header_inner = state.then_ignore(comma).then(number).then_ignore(comma).then(number);
    let header =
        des.ignore_then(header_inner.delimited_by(just('('), just(')'))).then_ignore(newline);

    let edge_inner = state.then_ignore(comma).then(label).then_ignore(comma).then(state);
    let edge = edge_inner.delimited_by(just('('), just(')'));
    let edges = edge.separated_by(newline);

    let parser = header.boxed().then_with(|((first_state, trans_count), states_count)| {
        edges.exactly(trans_count).map(move |edges| {
            let first_state = first_state.clone();
            let mut transitions = vec![Vec::new(); states_count];

            for ((start_state, label), end_state) in edges {
                transitions[start_state.0].push((label, end_state));
            }

            Lts { first_state, transitions }
        })
    });

    parser.then_ignore(end()).parse(source)
}
