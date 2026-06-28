//! # Galaxy map
//!
//! Tracks everything Jebediah has learned about the galaxy: which planets
//! exist, how many times he has visited each, and the adjacency between them.
//!
//! `unvisited()` returns a lazy iterator over planets Jeb knows about but has
//! never been to. Navigation uses this to prioritize unexplored destinations.

use common_game::utils::ID;
use std::collections::{HashMap, HashSet};

/// Running knowledge of the galaxy accumulated during exploration.
pub struct GalaxyMap {
    /// Visit count per known planet. A count of 0 means heard-about, not visited.
    visit_counts: HashMap<ID, u32>,
    /// Known adjacency: planet id -> set of neighbor ids.
    adjacency: HashMap<ID, HashSet<ID>>,
    starting_planet: ID,
}

impl GalaxyMap {
    /// Creates a new map seeded with the starting planet (visit count 1).
    pub fn new(starting_planet: ID) -> Self {
        let mut visit_counts = HashMap::new();
        visit_counts.insert(starting_planet, 1);
        Self {
            visit_counts,
            adjacency: HashMap::new(),
            starting_planet,
        }
    }

    /// Records one visit to `planet_id`, adding it to the map if unknown.
    pub fn visit(&mut self, planet_id: ID) {
        *self.visit_counts.entry(planet_id).or_insert(0) += 1;
        log::debug!("[JebMap] planet {} visited ({} times)", planet_id, self.visit_counts[&planet_id]);
    }

    /// Records the neighbor list for `planet_id`. Unknown neighbors are added
    /// with a visit count of 0 so they appear in `unvisited()`.
    pub fn record_neighbors(&mut self, planet_id: ID, neighbors: Vec<ID>) {
        for &n in &neighbors {
            self.visit_counts.entry(n).or_insert(0);
        }
        self.adjacency.insert(planet_id, neighbors.into_iter().collect());
    }

    /// Visit count for `planet_id`. Returns 0 for unknown planets.
    pub fn visit_count(&self, planet_id: ID) -> u32 {
        self.visit_counts.get(&planet_id).copied().unwrap_or(0)
    }

    /// Neighbors of `planet_id` that have been recorded so far.
    pub fn known_neighbors(&self, planet_id: ID) -> Vec<ID> {
        self.adjacency
            .get(&planet_id)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Total distinct planets known (visited or heard about).
    pub fn known_planet_count(&self) -> usize {
        self.visit_counts.len()
    }

    /// Planets with at least one visit recorded.
    pub fn visited_planet_count(&self) -> usize {
        self.visit_counts.values().filter(|&&c| c > 0).count()
    }

    /// Lazy iterator over known planets with zero visits.
    pub fn unvisited(&self) -> UnvisitedPlanets<'_> {
        UnvisitedPlanets {
            inner: self.visit_counts.iter(),
        }
    }

    pub fn log_summary(&self) -> String {
        format!(
            "known={} visited={} uncharted={} home={}",
            self.known_planet_count(),
            self.visited_planet_count(),
            self.known_planet_count().saturating_sub(self.visited_planet_count()),
            self.starting_planet,
        )
    }
}

/// Iterator over planets Jeb knows about but has never visited.
///
/// Created by [`GalaxyMap::unvisited`]. Borrows the map for its lifetime.
pub struct UnvisitedPlanets<'a> {
    inner: std::collections::hash_map::Iter<'a, ID, u32>,
}

impl<'a> Iterator for UnvisitedPlanets<'a> {
    type Item = ID;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (&planet_id, &count) = self.inner.next()?;
            if count == 0 {
                return Some(planet_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_map_has_starting_planet_visited_once() {
        let map = GalaxyMap::new(1);
        assert_eq!(map.visit_count(1), 1);
        assert_eq!(map.visited_planet_count(), 1);
    }

    #[test]
    fn recording_neighbors_adds_them_as_unvisited() {
        let mut map = GalaxyMap::new(1);
        map.record_neighbors(1, vec![2, 3, 4]);
        assert_eq!(map.known_planet_count(), 4);
        assert_eq!(map.visit_count(2), 0);
        assert_eq!(map.visit_count(3), 0);
    }

    #[test]
    fn unvisited_iterator_skips_visited_planets() {
        let mut map = GalaxyMap::new(1);
        map.record_neighbors(1, vec![2, 3, 4]);
        map.visit(2);

        let unvisited: Vec<ID> = map.unvisited().collect();
        assert!(!unvisited.contains(&1));
        assert!(!unvisited.contains(&2));
        assert!(unvisited.contains(&3));
        assert!(unvisited.contains(&4));
    }

    #[test]
    fn visit_increments_count() {
        let mut map = GalaxyMap::new(5);
        map.visit(5);
        map.visit(5);
        assert_eq!(map.visit_count(5), 3);
    }
}
