//! # Navigation
//!
//! Picks the next destination planet for Jebediah.
//! Strategy: prefer unvisited neighbors first; among visited, pick the
//! least-visited one; break ties randomly.
//!
//! TODO(Vivi): implement `pick_destination`.
//! - Use `map.unvisited()` to get unvisited planets, filter to known neighbors
//! - Fall back to the neighbor with lowest `map.visit_count()` if all visited
//! - Use the `rand` crate for random tiebreaking

use common_game::utils::ID;
use super::mapping::GalaxyMap;

/// Returns the next planet Jeb should travel to, or `None` if no neighbors
/// are known yet.
///
/// TODO(Vivi): implement.
pub fn pick_destination(_current_planet: ID, _map: &GalaxyMap) -> Option<ID> {
    None
}
