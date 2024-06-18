use chumsky::error::Simple;
use chumsky::primitive::{choice, just, none_of};
use chumsky::text::TextParser;
use chumsky::{text, Parser};
use solver::strategy::Player;

use crate::{Node, ParityGame};

pub fn parse_parity_game(source: &str) -> Result<ParityGame, Vec<Simple<char>>> {
    let parity = just("parity").padded();
    let number = text::int(10).map(|n: String| n.parse::<usize>().unwrap()).padded();
    let comma = just(',').padded();
    let semi = just(';');
    let newline = text::newline();

    let header = parity.then(number).then(semi).then(newline);

    let player = choice((just('0').to(Player::P0), just('1').to(Player::P1)));
    let successors = number.separated_by(comma);
    let comment = none_of(";").repeated();
    let row = number.then(number).then(player).then(successors).then_ignore(comment);
    let row = row.map(|(((id, relevance), player), successors)| Node {
        id,
        relevance,
        player,
        successors,
    });

    let rows = row.then_ignore(semi).separated_by(newline).allow_trailing();
    let game = header.ignore_then(rows).map(|nodes| ParityGame { nodes });

    game.parse(source)
}
