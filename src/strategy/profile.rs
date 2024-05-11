use std::cmp::Ordering;

use super::game::{Game, NodeId, Player, Relevance, Reward};

#[derive(Clone, Default)]
pub struct PlayProfile {
    /// Most relevant node of the cycle.
    pub most_relevant: NodeId,
    /// Nodes more relevant visited before the cycle, sorted by most relevant first.
    pub relevant_before: Vec<NodeId>,
    /// Number of nodes visited before the most relevant of the cycle.
    pub count_before: usize,
}

impl PlayProfile {
    pub fn cmp<'a>(&'a self, that: &'a PlayProfile, gr: impl GetRelevance) -> Ordering {
        // Compare the most relevant vertex of the cycle
        if self.most_relevant != that.most_relevant {
            let this_rew = gr.relevance_of(self.most_relevant).reward();
            let that_rew = gr.relevance_of(that.most_relevant).reward();
            return Ord::cmp(&this_rew, &that_rew);
        }

        // Compare the set of more relevant nodes visited before the cycle
        let rewards_before = |p: &'a PlayProfile| {
            p.relevant_before
                .iter()
                .map(|&u| gr.relevance_of(u).reward())
                .chain([Reward::Neutral])
        };
        match Iterator::cmp(rewards_before(self), rewards_before(that)) {
            Ordering::Equal => {}
            ordering => return ordering,
        }

        // Compare the number of nodes visited before most relevant vertex of the loop
        match gr.relevance_of(self.most_relevant).player() {
            // If P0 is winning a shorter path is better (order is reversed, less is greater).
            Player::P0 => Ord::cmp(&self.count_before, &that.count_before).reverse(),
            // If P0 is losing a longer path is better (order is normal).
            Player::P1 => Ord::cmp(&self.count_before, &that.count_before),
        }
    }
}

pub trait GetRelevance {
    fn relevance_of(&self, u: NodeId) -> Relevance;
}

impl<'a> GetRelevance for &'a Game {
    fn relevance_of(&self, u: NodeId) -> Relevance {
        (*self).relevance_of(u)
    }
}

impl<F: Fn(NodeId) -> Relevance> GetRelevance for F {
    fn relevance_of(&self, u: NodeId) -> Relevance {
        self(u)
    }
}
