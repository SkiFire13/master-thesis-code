use crate::solver::Player;

use super::profile::PlayProfile;
use super::{SNodeId, StrategySolver};

impl StrategySolver {
    pub(super) fn update_strategy(&mut self, u: SNodeId, v: SNodeId) {
        let old = self.strategy[u];
        if old != v {
            self.strategy[u] = v;
            self.inverse_strategy[old].swap_remove(&u);
            self.inverse_strategy[v].insert(u);
        }
    }

    pub(super) fn improve(&mut self) -> bool {
        let mut improved = false;

        for u in self.strategy.indexes() {
            let Player::P0 = self.players[u] else { continue };

            let v = self.strategy[u];
            let mut best = v;
            for &w in self.succs[u].iter().filter(|&&w| w != v) {
                if PlayProfile::compare(self, u, best, w).is_lt() {
                    best = w;
                    improved = true;
                }
            }

            if improved {
                self.update_strategy(u, best);
            }
        }

        improved
    }
}
