use std::cmp::Ordering;

use crate::solver::Player;

use super::{Reward, SNodeId, StrategySolver};

#[derive(Clone, Debug, Default)]
pub struct PlayProfile {
    /// Most relevant node of the cycle.
    pub most_relevant: SNodeId,
    /// Nodes more relevant visited before the cycle, sorted by most relevant first.
    pub relevant_before: Vec<SNodeId>,
    /// Number of nodes visited before the most relevant of the cycle.
    pub count_before: usize,
}

impl PlayProfile {
    pub fn losing_for_player(player: Player) -> Self {
        let most_relevant = match player {
            Player::P0 => SNodeId::W1,
            Player::P1 => SNodeId::W0,
        };

        Self { most_relevant, relevant_before: Vec::new(), count_before: 0 }
    }

    pub fn winner(&self, game: &StrategySolver) -> Player {
        game.relevance_of(self.most_relevant).winner()
    }

    fn rewards_before<'a>(&'a self, game: &'a StrategySolver) -> impl Iterator<Item = Reward> + 'a {
        self.relevant_before.iter().map(move |&u| game.reward_of(u)).chain([Reward::Neutral])
    }

    fn cmp_w(&self, that: &PlayProfile, game: &StrategySolver) -> Ordering {
        // Compare the most relevant vertex of the cycle
        let cmp_most_relevant = || {
            let this_rew = game.reward_of(self.most_relevant);
            let that_rew = game.reward_of(that.most_relevant);
            Ord::cmp(&this_rew, &that_rew)
        };

        // Compare the set of more relevant nodes visited before the cycle.
        // This should ignore all those nodes with relevance less than w,
        // but if most_relevant compare equal then we know that's equal to w
        // and thus all these have relevance bigger than w.
        let cmp_relevant_before =
            || Iterator::cmp(self.rewards_before(game), that.rewards_before(game));

        cmp_most_relevant().then_with(cmp_relevant_before)
    }

    fn cmp(&self, that: &PlayProfile, game: &StrategySolver) -> Ordering {
        // Compare the number of nodes visited before most relevant vertex of the loop
        let cmp_count_before = || match self.winner(game) {
            // If P0 is winning a shorter path is better (order is reversed, less is greater).
            Player::P0 => Ord::cmp(&self.count_before, &that.count_before).reverse(),
            // If P0 is losing a longer path is better (order is normal).
            Player::P1 => Ord::cmp(&self.count_before, &that.count_before),
        };

        self.cmp_w(that, game).then_with(cmp_count_before)
    }

    // Compares the play profiles of n1 and n2 in the context of the successors of n0.
    // This will do either a normal comparison or one that ignores the
    pub fn compare(game: &StrategySolver, n0: SNodeId, n1: SNodeId, n2: SNodeId) -> Ordering {
        match game.profiles[n0].most_relevant == n0 {
            true => game.profiles[n1].cmp_w(&game.profiles[n2], game),
            false => game.profiles[n1].cmp(&game.profiles[n2], game),
        }
    }
}
