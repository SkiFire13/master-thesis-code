use std::cmp::Ordering;

use super::{GetRelevance, NodeId, Player, Reward};

#[derive(Clone, Debug, Default)]
pub struct PlayProfile {
    /// Most relevant node of the cycle.
    pub most_relevant: NodeId,
    /// Nodes more relevant visited before the cycle, sorted by most relevant first.
    pub relevant_before: Vec<NodeId>,
    /// Number of nodes visited before the most relevant of the cycle.
    pub count_before: usize,
}

impl PlayProfile {
    pub fn winning(&self, gr: &impl GetRelevance) -> Player {
        gr.relevance_of(self.most_relevant).player()
    }

    fn rewards_before<'a>(
        &'a self,
        gr: &'a impl GetRelevance,
    ) -> impl Iterator<Item = Reward> + 'a {
        self.relevant_before.iter().map(move |&u| gr.reward_of(u)).chain([Reward::Neutral])
    }

    pub fn cmp(&self, that: &PlayProfile, gr: &impl GetRelevance) -> Ordering {
        // Compare the most relevant vertex of the cycle
        let cmp_most_relevant = || {
            let this_rew = gr.reward_of(self.most_relevant);
            let that_rew = gr.reward_of(that.most_relevant);
            Ord::cmp(&this_rew, &that_rew)
        };

        // Compare the set of more relevant nodes visited before the cycle
        let cmp_relevant_before =
            || Iterator::cmp(self.rewards_before(gr), that.rewards_before(gr));

        // Compare the number of nodes visited before most relevant vertex of the loop
        let cmp_count_before = || match self.winning(gr) {
            // If P0 is winning a shorter path is better (order is reversed, less is greater).
            Player::P0 => Ord::cmp(&self.count_before, &that.count_before).reverse(),
            // If P0 is losing a longer path is better (order is normal).
            Player::P1 => Ord::cmp(&self.count_before, &that.count_before),
        };

        cmp_most_relevant().then_with(cmp_relevant_before).then_with(cmp_count_before)
    }
}
