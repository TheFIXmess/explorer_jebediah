//! # Navigation
//!
//! Picks the next destination planet for Jebediah.
//! Strategy: prefer unvisited neighbors first; among visited, pick the
//! least-visited one.

use common_game::utils::ID;
use super::mapping::GalaxyMap;

/// Returns the next planet Jeb should travel to, or `None` if no neighbors
/// are known yet.
pub fn pick_destination(current_planet: ID, map: &GalaxyMap) -> Option<ID> {
    let neighbors = map.known_neighbors(current_planet);

    if neighbors.is_empty() {
        return None;
    }

    // First priority: known neighbor never visited.
    if let Some(dst) = neighbors
        .iter()
        .copied()
        .filter(|planet_id| map.visit_count(*planet_id) == 0)
        .min()
    {
        return Some(dst);
    }

    // Fallback: least visited neighbor.
    neighbors
        .into_iter()
        .min_by_key(|planet_id| map.visit_count(*planet_id))
}