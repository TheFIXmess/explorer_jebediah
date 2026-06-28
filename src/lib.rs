//! # Explorer Jebediah
//!
//! Jebediah is the reckless cartographer. He prioritizes unvisited planets,
//! ignores danger, and maps everything he encounters. He collects resources
//! opportunistically but navigation always wins over collection.
//!
//! The orchestrator calls [`create_explorer`] once at spawn time and then
//! interacts exclusively through the channel protocol defined in common_game.

pub mod explorer;

pub use explorer::JebExplorer;

use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::components::resource::ResourceType;
use crossbeam_channel::{Receiver, Sender};

/// Bag content type reported to the orchestrator.
/// Matches the protocol generic: `ExplorerToOrchestrator<Vec<(ResourceType, usize)>>`.
pub type BagSummary = Vec<(ResourceType, usize)>;

/// Creates a Jebediah explorer instance ready to be handed to a thread.
///
/// - `rx_planet` is permanent: the orchestrator retains the matching sender
///   and passes it to each new planet when the explorer moves.
/// - `tx_planet` points to the starting planet; it is replaced on each move.
pub fn create_explorer(
    id: u32,
    rx_orchestrator: Receiver<OrchestratorToExplorer>,
    tx_orchestrator: Sender<ExplorerToOrchestrator<BagSummary>>,
    rx_planet: Receiver<PlanetToExplorer>,
    tx_planet: Sender<ExplorerToPlanet>,
    starting_planet: u32,
) -> Result<JebExplorer, String> {
    JebExplorer::new(
        id,
        rx_orchestrator,
        tx_orchestrator,
        rx_planet,
        tx_planet,
        starting_planet,
    )
}
