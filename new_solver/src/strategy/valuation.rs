use std::collections::{HashMap, VecDeque};

use either::Either;

use crate::solver::Player;
use crate::Set;

use super::{SNodeId, StrategySolver};

impl StrategySolver {
    fn strat_preds(&self, u: SNodeId) -> impl Iterator<Item = SNodeId> + '_ {
        match self.players[u] {
            Player::P0 => self.preds[u].iter().copied(),
            Player::P1 => self.inverse_strategy[u].iter().copied(),
        }
    }

    fn strat_succs(&self, u: SNodeId) -> impl Iterator<Item = SNodeId> + '_ {
        match self.players[u] {
            Player::P0 => Either::Left(std::iter::once(self.strategy[u])),
            Player::P1 => Either::Right(self.succs[u].iter().copied()),
        }
    }

    fn eval_preds(&self, u: SNodeId) -> impl Iterator<Item = SNodeId> + '_ {
        self.strat_preds(u).filter(|&v| !self.evaluated[v])
    }

    pub(super) fn valuation(&mut self) {
        self.evaluated.clear();
        self.evaluated.resize(self.players.len(), false);

        let mut nodes = self.players.indexes().collect::<Vec<_>>();
        nodes.sort_by_key(|&n| self.reward_of(n));

        for w in nodes {
            if self.evaluated[w] {
                continue;
            }

            let rel_of = |v| self.relevance_of(v);
            let rel_of_w = rel_of(w);
            let reach_set = reach(w, |u| self.eval_preds(u).filter(|&v| rel_of(v) <= rel_of_w));

            if !self.strat_succs(w).any(|v| reach_set.contains(&v)) {
                continue;
            }

            let mut k_set = reach(w, |u| self.eval_preds(u));
            SubvaluationGame::new(self, w, &mut k_set).subvaluation();

            for v in k_set {
                self.evaluated[v] = true;
            }
        }
    }
}

struct SubvaluationGame<'a> {
    game: &'a mut StrategySolver,
    w: SNodeId,
    k_set: &'a Set<SNodeId>,
    removed_edges: Set<(SNodeId, SNodeId)>,
    removed_succs: rustc_hash::FxHashMap<SNodeId, usize>,
}

impl<'a> SubvaluationGame<'a> {
    fn new(game: &'a mut StrategySolver, w: SNodeId, k_set: &'a mut Set<SNodeId>) -> Self {
        k_set.sort_unstable_by(|&v, &u| game.relevance_of(v).cmp(&game.relevance_of(u)));

        Self { game, w, k_set, removed_edges: Set::default(), removed_succs: HashMap::default() }
    }

    fn preds(&self, v: SNodeId) -> impl Iterator<Item = SNodeId> + '_ {
        self.game
            .strat_preds(v)
            .filter(|u| self.k_set.contains(u))
            .filter(move |&u| !self.removed_edges.contains(&(u, v)))
    }

    fn succs(&self, u: SNodeId) -> impl Iterator<Item = SNodeId> + '_ {
        self.game
            .strat_succs(u)
            .filter(|v| self.k_set.contains(v))
            .filter(move |&v| !self.removed_edges.contains(&(u, v)))
    }

    fn subvaluation(&mut self) {
        let rel_of_w = self.game.relevance_of(self.w);

        for &v in self.k_set {
            self.game.profiles[v].most_relevant = self.w;
            self.game.profiles[v].relevant_before.clear();
        }

        for &u in self.k_set.iter().rev() {
            if self.game.relevance_of(u) <= rel_of_w {
                break;
            }

            match self.game.relevance_of(u).winner() {
                Player::P0 => self.prevent_paths(u),
                Player::P1 => self.force_paths(u),
            }
        }

        match self.game.relevance_of(self.w).winner() {
            Player::P0 => self.set_maximal_distances(),
            Player::P1 => self.set_minimal_distances(),
        }
    }

    fn prevent_paths(&mut self, u: SNodeId) {
        // Find nodes that can reach w without going through u.
        let u_set = reach(self.w, |v| self.preds(v).filter(|&v| v != u));

        // Update profiles of nodes whose path must go through u.
        for &v in self.k_set.iter().filter(|&v| !u_set.contains(v)) {
            self.game.profiles[v].relevant_before.push(u);
        }

        // Remove edges that would make paths go through u when it's possible
        // to avoid it, that is edges from u_nodes U {u} to V \ U_nodes.
        for &v in u_set.iter().chain([&u]) {
            for &s in self.game.succs[v].iter().filter(|&s| !u_set.contains(s)) {
                if self.k_set.contains(&s) && self.removed_edges.insert((v, s)) {
                    *self.removed_succs.entry(v).or_insert(0) += 1;
                }
            }
        }
    }

    fn force_paths(&mut self, u: SNodeId) {
        // Find nodes that can reach u without going through w.
        let u_set = reach(u, |n| self.preds(n).filter(|&v| v != self.w));

        // Update profiles of nodes whose path can go through u.
        for &v in &u_set {
            self.game.profiles[v].relevant_before.push(u);
        }

        // Remove edges that would make paths not go through u when it's possible
        // to do so, that is edges from u_nodes \ {u} to K \ u_nodes
        for &v in u_set.iter().filter(|&&v| v != u) {
            for &s in self.game.succs[v].iter().filter(|&s| !u_set.contains(s)) {
                if self.k_set.contains(&s) && self.removed_edges.insert((v, s)) {
                    *self.removed_succs.entry(v).or_insert(0) += 1;
                }
            }
        }
    }

    fn succs_count_of(&self, v: SNodeId) -> usize {
        self.game.strat_succs(v).filter(|u| self.k_set.contains(u)).count()
            - self.removed_succs.get(&v).unwrap_or(&0)
    }

    fn set_maximal_distances(&mut self) {
        let mut remaining_successors = self
            .k_set
            .iter()
            .map(|&v| (v, self.succs_count_of(v)))
            .collect::<rustc_hash::FxHashMap<_, _>>();
        let mut queue = VecDeque::from([(self.w, self.succs(self.w).next().unwrap(), 0)]);

        while let Some((v, succ, d)) = queue.pop_front() {
            self.game.profiles[v].count_before = d;
            self.game.update_strategy(v, succ);

            for u in self.preds(v).filter(|&u| u != self.w) {
                // Decrease number of remaining successors to visit
                let remaining = remaining_successors.get_mut(&u).unwrap();
                *remaining -= 1;

                // If last was visited then add node to the queue with one more edge.
                if *remaining == 0 {
                    queue.push_back((u, v, d + 1));
                }
            }
        }
    }

    fn set_minimal_distances(&mut self) {
        let mut seen = Set::default();
        let mut queue = VecDeque::from([(self.w, self.succs(self.w).next().unwrap(), 0)]);

        // Backward BFS
        while let Some((v, succ, d)) = queue.pop_front() {
            if seen.insert(v) {
                self.game.profiles[v].count_before = d;
                self.game.update_strategy(v, succ);
                queue.extend(self.preds(v).map(|u| (u, v, d + 1)))
            }
        }
    }
}

fn reach<F, I>(start: SNodeId, mut explore: F) -> Set<SNodeId>
where
    F: FnMut(SNodeId) -> I,
    I: Iterator<Item = SNodeId>,
{
    let mut stack = vec![start];
    let mut set = Set::from_iter([start]);

    // DFS according to explore
    while let Some(node) = stack.pop() {
        stack.extend(explore(node).filter(|&next| set.insert(next)));
    }

    set
}
