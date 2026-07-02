//! # Navigation
//!
//! Picks the next destination planet for Jebediah.
//!
//! Priority:
//! 1. If there is an unvisited neighbor, go there immediately.
//! 2. Otherwise, find the shortest known path to the nearest unvisited planet.
//! 3. If all known planets are already visited, keep moving to the least-visited neighbor.
//!
//! This keeps Jebediah focused on fast exploration.

use common_game::utils::ID;
use std::collections::{HashMap, HashSet, VecDeque};

use super::mapping::GalaxyMap;

/// Returns the next planet Jeb should travel to.
///
/// Important:
/// this returns the **next hop**, not necessarily the final target.
///
/// Example:
///
/// ```text
/// current = 1
/// unvisited target = 4
/// known path = 1 -> 2 -> 3 -> 4
/// returns 2
/// ```
pub fn pick_destination(current_planet: ID, map: &GalaxyMap) -> Option<ID> {
    let neighbors = map.known_neighbors(current_planet);

    if neighbors.is_empty() {
        return None;
    }

    // 1. Fastest case: directly move to an unvisited neighbor.
    if let Some(destination) = pick_unvisited_neighbor(&neighbors, map) {
        return Some(destination);
    }

    // 2. If no direct neighbor is unvisited, search the known map for the
    // shortest path to any known unvisited planet.
    if let Some(destination) = next_hop_towards_nearest_unvisited(current_planet, map) {
        return Some(destination);
    }

    // 3. If everything known is already visited, keep moving.
    // Pick the least visited neighbor to avoid getting stuck in a tiny loop.
    pick_least_visited_neighbor(neighbors, map)
}

/// Prefer an immediately reachable unvisited planet.
fn pick_unvisited_neighbor(neighbors: &[ID], map: &GalaxyMap) -> Option<ID> {
    neighbors
        .iter()
        .copied()
        .filter(|planet_id| !map.is_visited(*planet_id))
        .min()
}

/// Finds the next hop toward the nearest known unvisited planet using BFS.
///
/// Returns `None` if no unvisited known planet is reachable through the known map.
fn next_hop_towards_nearest_unvisited(
    current_planet: ID,
    map: &GalaxyMap,
) -> Option<ID> {
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    let mut previous: HashMap<ID, ID> = HashMap::new();

    queue.push_back(current_planet);
    visited.insert(current_planet);

    while let Some(planet) = queue.pop_front() {
        if planet != current_planet && !map.is_visited(planet) {
            return reconstruct_next_hop(
                current_planet,
                planet,
                &previous,
            );
        }

        for neighbor in map.known_neighbors(planet) {
            if visited.insert(neighbor) {
                previous.insert(neighbor, planet);
                queue.push_back(neighbor);
            }
        }
    }

    None
}

/// Reconstructs only the first step from `start` toward `target`.
fn reconstruct_next_hop(
    start: ID,
    target: ID,
    previous: &HashMap<ID, ID>,
) -> Option<ID> {
    let mut current = target;

    while let Some(&parent) = previous.get(&current) {
        if parent == start {
            return Some(current);
        }

        current = parent;
    }

    None
}

/// If all known planets are already visited, keep moving through the graph.
///
/// Strategy:
/// choose the neighbor with the lowest visit count.
/// If two neighbors have the same visit count, choose the smaller ID for stable behavior.
fn pick_least_visited_neighbor(
    neighbors: Vec<ID>,
    map: &GalaxyMap,
) -> Option<ID> {
    neighbors
        .into_iter()
        .min_by_key(|planet_id| {
            (
                map.visit_count(*planet_id),
                *planet_id,
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_unvisited_neighbor_first() {
        let mut map = GalaxyMap::new(1);
        map.record_neighbors(1, vec![2, 3]);

        let destination = pick_destination(1, &map);

        assert_eq!(destination, Some(2));
    }

    #[test]
    fn picks_nearest_unvisited_through_known_path() {
        let mut map = GalaxyMap::new(1);

        map.record_neighbors(1, vec![2]);
        map.visit(2);

        map.record_neighbors(2, vec![1, 3]);
        map.visit(3);

        map.record_neighbors(3, vec![2, 4]);

        let destination = pick_destination(1, &map);

        assert_eq!(destination, Some(2));
    }

    #[test]
    fn picks_least_visited_neighbor_when_everything_is_visited() {
        let mut map = GalaxyMap::new(1);

        map.record_neighbors(1, vec![2, 3]);

        map.visit(2);
        map.visit(2);

        map.visit(3);

        let destination = pick_destination(1, &map);

        assert_eq!(destination, Some(3));
    }

    #[test]
    fn returns_none_when_no_neighbors_are_known() {
        let map = GalaxyMap::new(1);

        let destination = pick_destination(1, &map);

        assert_eq!(destination, None);
    }

    #[test]
    fn chooses_stable_lowest_id_when_unvisited_neighbors_tie() {
        let mut map = GalaxyMap::new(1);

        map.record_neighbors(1, vec![4, 2, 3]);

        let destination = pick_destination(1, &map);

        assert_eq!(destination, Some(2));
    }
}