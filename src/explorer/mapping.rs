//! # Galaxy map
//!
//! Tracks everything Jebediah has learned about the galaxy:
//! - which planets exist;
//! - how many times he has visited each planet;
//! - bilateral adjacency between planets;
//! - starting planet;
//! - current exploration progress.
//!
//! Jebediah's priority is fast exploration. The map therefore keeps enough
//! information to help navigation prefer unvisited planets and still move
//! through already visited planets when needed.

use common_game::utils::ID;
use std::collections::{HashMap, HashSet};

/// Running knowledge of the galaxy accumulated during exploration.
pub struct GalaxyMap {
    /// Visit count per known planet.
    ///
    /// A count of 0 means the planet is known because it appeared as a neighbor,
    /// but Jebediah has never visited it.
    visit_counts: HashMap<ID, u32>,

    /// Known bilateral adjacency.
    ///
    /// If we discover `A -> B`, we also store `B -> A`.
    adjacency: HashMap<ID, HashSet<ID>>,

    /// First planet where Jebediah spawned.
    starting_planet: ID,
}

impl GalaxyMap {
    /// Creates a new map seeded with the starting planet.
    ///
    /// The starting planet is considered already visited once.
    pub fn new(starting_planet: ID) -> Self {
        let mut visit_counts = HashMap::new();
        visit_counts.insert(starting_planet, 1);

        let mut adjacency = HashMap::new();
        adjacency.insert(starting_planet, HashSet::new());

        Self {
            visit_counts,
            adjacency,
            starting_planet,
        }
    }

    /// Records one visit to `planet_id`, adding it to the map if unknown.
    pub fn visit(&mut self, planet_id: ID) {
        let count = self.visit_counts.entry(planet_id).or_insert(0);
        *count += 1;

        self.adjacency.entry(planet_id).or_default();

        log::debug!(
            "[JebMap] planet {} visited ({} times)",
            planet_id,
            self.visit_count(planet_id)
        );
    }

    /// Records the neighbor list for `planet_id`.
    ///
    /// Important:
    /// this stores connections bilaterally.
    ///
    /// Example:
    ///
    /// ```text
    /// record_neighbors(3, vec![2, 4])
    /// ```
    ///
    /// stores:
    ///
    /// ```text
    /// 3 -> 2
    /// 2 -> 3
    /// 3 -> 4
    /// 4 -> 3
    /// ```
    ///
    /// Unknown neighbors are added with visit count 0 so they appear in
    /// `unvisited()`.
    pub fn record_neighbors(&mut self, planet_id: ID, neighbors: Vec<ID>) {
        self.visit_counts.entry(planet_id).or_insert(0);
        self.adjacency.entry(planet_id).or_default();

        for neighbor in neighbors {
            self.visit_counts.entry(neighbor).or_insert(0);
            self.adjacency.entry(neighbor).or_default();

            self.adjacency
                .entry(planet_id)
                .or_default()
                .insert(neighbor);

            self.adjacency
                .entry(neighbor)
                .or_default()
                .insert(planet_id);
        }

        log::debug!(
            "[JebMap] neighbors recorded for planet {} | {}",
            planet_id,
            self.log_summary()
        );
    }

    /// Visit count for `planet_id`.
    ///
    /// Returns 0 for unknown planets.
    pub fn visit_count(&self, planet_id: ID) -> u32 {
        self.visit_counts.get(&planet_id).copied().unwrap_or(0)
    }

    /// Returns true if the planet is known by the map.
    pub fn knows_planet(&self, planet_id: ID) -> bool {
        self.visit_counts.contains_key(&planet_id)
    }

    /// Returns true if the planet has been visited at least once.
    pub fn is_visited(&self, planet_id: ID) -> bool {
        self.visit_count(planet_id) > 0
    }

    /// Neighbors of `planet_id` that have been recorded so far.
    ///
    /// The output is sorted to make behavior and logs more deterministic.
    pub fn known_neighbors(&self, planet_id: ID) -> Vec<ID> {
        let mut neighbors: Vec<ID> = self
            .adjacency
            .get(&planet_id)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default();

        neighbors.sort_unstable();
        neighbors
    }

    /// Total distinct planets known, visited or only discovered as neighbors.
    pub fn known_planet_count(&self) -> usize {
        self.visit_counts.len()
    }

    /// Planets with at least one visit recorded.
    pub fn visited_planet_count(&self) -> usize {
        self.visit_counts
            .values()
            .filter(|&&count| count > 0)
            .count()
    }

    /// Planets known but never visited.
    pub fn unvisited_planet_count(&self) -> usize {
        self.visit_counts
            .values()
            .filter(|&&count| count == 0)
            .count()
    }

    /// Returns true if every known planet has been visited at least once.
    pub fn all_known_planets_visited(&self) -> bool {
        self.unvisited_planet_count() == 0
    }

    /// Lazy iterator over known planets with zero visits.
    pub fn unvisited(&self) -> UnvisitedPlanets<'_> {
        UnvisitedPlanets {
            inner: self.visit_counts.iter(),
        }
    }

    /// Returns the known unvisited planets as a sorted vector.
    pub fn unvisited_planets(&self) -> Vec<ID> {
        let mut planets: Vec<ID> = self.unvisited().collect();
        planets.sort_unstable();
        planets
    }

    /// Returns visited planets as a sorted vector.
    pub fn visited_planets(&self) -> Vec<ID> {
        let mut planets: Vec<ID> = self
            .visit_counts
            .iter()
            .filter_map(|(&planet_id, &count)| {
                if count > 0 {
                    Some(planet_id)
                } else {
                    None
                }
            })
            .collect();

        planets.sort_unstable();
        planets
    }

    /// Returns known planets as a sorted vector.
    pub fn known_planets(&self) -> Vec<ID> {
        let mut planets: Vec<ID> = self.visit_counts.keys().copied().collect();
        planets.sort_unstable();
        planets
    }

    /// Returns true if two planets are known as connected.
    pub fn are_connected(&self, a: ID, b: ID) -> bool {
        self.adjacency
            .get(&a)
            .map(|neighbors| neighbors.contains(&b))
            .unwrap_or(false)
    }

    /// Human-readable adjacency summary.
    pub fn adjacency_summary(&self) -> String {
        let mut entries: Vec<(ID, Vec<ID>)> = self
            .adjacency
            .iter()
            .map(|(&planet_id, neighbors)| {
                let mut list: Vec<ID> = neighbors.iter().copied().collect();
                list.sort_unstable();
                (planet_id, list)
            })
            .collect();

        entries.sort_by_key(|(planet_id, _)| *planet_id);

        format!("{entries:?}")
    }

    /// Human-readable visit count summary.
    pub fn visit_counts_summary(&self) -> String {
        let mut entries: Vec<(ID, u32)> = self
            .visit_counts
            .iter()
            .map(|(&planet_id, &count)| (planet_id, count))
            .collect();

        entries.sort_by_key(|(planet_id, _)| *planet_id);

        format!("{entries:?}")
    }

    /// Compact summary for logs.
    pub fn log_summary(&self) -> String {
        format!(
            "known={} visited={} uncharted={} home={} visits={} adjacency={}",
            self.known_planet_count(),
            self.visited_planet_count(),
            self.unvisited_planet_count(),
            self.starting_planet,
            self.visit_counts_summary(),
            self.adjacency_summary(),
        )
    }
}

/// Iterator over planets Jeb knows about but has never visited.
///
/// Created by [`GalaxyMap::unvisited`].
/// Borrows the map for its lifetime.
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
        assert_eq!(map.known_planet_count(), 1);
        assert!(map.is_visited(1));
    }

    #[test]
    fn recording_neighbors_adds_them_as_unvisited() {
        let mut map = GalaxyMap::new(1);

        map.record_neighbors(1, vec![2, 3, 4]);

        assert_eq!(map.known_planet_count(), 4);
        assert_eq!(map.visit_count(2), 0);
        assert_eq!(map.visit_count(3), 0);
        assert_eq!(map.visit_count(4), 0);

        assert!(!map.is_visited(2));
        assert!(!map.is_visited(3));
        assert!(!map.is_visited(4));
    }

    #[test]
    fn recording_neighbors_stores_connections_bilaterally() {
        let mut map = GalaxyMap::new(1);

        map.record_neighbors(1, vec![2, 3]);

        assert!(map.are_connected(1, 2));
        assert!(map.are_connected(2, 1));

        assert!(map.are_connected(1, 3));
        assert!(map.are_connected(3, 1));
    }

    #[test]
    fn known_neighbors_are_sorted() {
        let mut map = GalaxyMap::new(1);

        map.record_neighbors(1, vec![4, 2, 3]);

        assert_eq!(map.known_neighbors(1), vec![2, 3, 4]);
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

    #[test]
    fn all_known_planets_visited_returns_true_only_when_no_uncharted_planets_exist() {
        let mut map = GalaxyMap::new(1);

        assert!(map.all_known_planets_visited());

        map.record_neighbors(1, vec![2]);

        assert!(!map.all_known_planets_visited());

        map.visit(2);

        assert!(map.all_known_planets_visited());
    }

    #[test]
    fn visited_planets_are_sorted() {
        let mut map = GalaxyMap::new(3);

        map.record_neighbors(3, vec![5, 1]);
        map.visit(5);
        map.visit(1);

        assert_eq!(map.visited_planets(), vec![1, 3, 5]);
    }

    #[test]
    fn unvisited_planets_are_sorted() {
        let mut map = GalaxyMap::new(3);

        map.record_neighbors(3, vec![5, 1, 2]);
        map.visit(1);

        assert_eq!(map.unvisited_planets(), vec![2, 5]);
    }
}