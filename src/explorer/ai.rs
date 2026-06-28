//! # Jebediah's core AI
//!
//! State machine for the explorer using the typestate pattern: the current state
//! (`Idle`, `OnPlanet`, `Traveling`) is encoded as a generic type parameter so
//! invalid transitions are caught at compile time rather than at runtime.
//!
//! The orchestrator only interacts with the `JebExplorer` type alias, which
//! fixes the state to `Idle` at construction. All state transitions happen
//! internally inside `run()`.

use std::marker::PhantomData;

use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};

use super::mapping::GalaxyMap;
use super::mood::{Mood, JebEvent};
use crate::BagSummary;

// Zero-sized state marker types. No runtime cost; used only by the compiler.

/// Explorer is waiting for the orchestrator to start it.
pub struct Idle;

/// Explorer is on a planet and can interact with it.
pub struct OnPlanet {
    pub planet_id: ID,
}

/// Explorer is in transit between two planets.
pub struct Traveling {
    pub destination: ID,
}

// Sealed trait: only the three states above can be used as the generic parameter.
// External code cannot introduce new states.
mod private {
    pub trait Sealed {}
    impl Sealed for super::Idle {}
    impl Sealed for super::OnPlanet {}
    impl Sealed for super::Traveling {}
}

/// Marker trait for valid explorer states.
pub trait State: private::Sealed {}
impl State for Idle {}
impl State for OnPlanet {}
impl State for Traveling {}

/// Jebediah Kerman — the reckless cartographer.
///
/// The generic `S` encodes the current state. The orchestrator uses the
/// [`JebExplorer`] alias and never interacts with the generic parameter directly.
pub struct JebExplorerInner<S: State> {
    pub(crate) id: ID,
    pub(crate) rx_orchestrator: Receiver<OrchestratorToExplorer>,
    pub(crate) tx_orchestrator: Sender<ExplorerToOrchestrator<BagSummary>>,
    /// Permanent receiver for planet messages. Created once at spawn; the sender
    /// is handed to each new planet when the explorer arrives.
    pub(crate) rx_planet: Receiver<PlanetToExplorer>,
    /// Sender to the current planet. Replaced on every move.
    pub(crate) tx_planet: Sender<ExplorerToPlanet>,
    pub(crate) map: GalaxyMap,
    pub(crate) mood: Mood,
    pub(crate) alive: bool,
    pub(crate) _state: PhantomData<S>,
    pub(crate) current_planet: Option<ID>,
    pub(crate) destination: Option<ID>,
}

/// Public handle. Use [`crate::create_explorer`] to construct.
pub type JebExplorer = JebExplorerInner<Idle>;

impl JebExplorerInner<Idle> {
    pub fn new(
        id: u32,
        rx_orchestrator: Receiver<OrchestratorToExplorer>,
        tx_orchestrator: Sender<ExplorerToOrchestrator<BagSummary>>,
        rx_planet: Receiver<PlanetToExplorer>,
        tx_planet: Sender<ExplorerToPlanet>,
        starting_planet: u32,
    ) -> Result<Self, String> {
        Ok(Self {
            id,
            rx_orchestrator,
            tx_orchestrator,
            rx_planet,
            tx_planet,
            map: GalaxyMap::new(starting_planet),
            mood: Mood::new(id),
            alive: true,
            _state: PhantomData,
            current_planet: Some(starting_planet),
            destination: None,
        })
    }

    /// Blocks on the orchestrator channel, handling messages until killed.
    pub fn run(mut self) -> Result<(), String> {
        log::info!("[Jeb #{}] Engines on. Let's see what's out there.", self.id);

        loop {
            match self.rx_orchestrator.recv() {
                Ok(msg) => {
                    if !self.handle_orchestrator_message(msg) {
                        break;
                    }
                }
                Err(_) => {
                    log::warn!("[Jeb #{}] Orchestrator channel closed.", self.id);
                    break;
                }
            }
        }

        log::info!("[Jeb #{}] Going dark.", self.id);
        Ok(())
    }

    /// Returns `false` when the explorer should stop (i.e. on `KillExplorer`).
    fn handle_orchestrator_message(&mut self, msg: OrchestratorToExplorer) -> bool {
        use OrchestratorToExplorer::*;
        use ExplorerToOrchestrator::*;

        match msg {
            StartExplorerAI => {
                log::info!("[Jeb #{}] {}", self.id, self.mood.on_event(JebEvent::Started));
                let _ = self.tx_orchestrator.send(StartExplorerAIResult {
                    explorer_id: self.id,
                });
            }

            StopExplorerAI => {
                log::info!("[Jeb #{}] Stopping.", self.id);
                let _ = self.tx_orchestrator.send(StopExplorerAIResult {
                    explorer_id: self.id,
                });
            }

            KillExplorer => {
                log::info!("[Jeb #{}] {}", self.id, self.mood.on_event(JebEvent::Killed));
                let _ = self.tx_orchestrator.send(KillExplorerResult {
                    explorer_id: self.id,
                });
                return false;
            }

            ResetExplorerAI => {
                self.map = GalaxyMap::new(self.current_planet.unwrap_or(0));
                self.mood = Mood::new(self.id);
                log::info!("[Jeb #{}] Reset.", self.id);
                let _ = self.tx_orchestrator.send(ResetExplorerAIResult {
                    explorer_id: self.id,
                });
            }

            MoveToPlanet { sender_to_new_planet, planet_id } => {
                if let Some(new_tx) = sender_to_new_planet {
                    self.tx_planet = new_tx;
                }
                self.current_planet = Some(planet_id);
                self.map.visit(planet_id);
                log::info!("[Jeb #{}] {} Arrived at planet {}.", self.id, self.mood.on_event(JebEvent::Arrived(planet_id)), planet_id);
                let _ = self.tx_orchestrator.send(MovedToPlanetResult {
                    explorer_id: self.id,
                    planet_id,
                });
            }

            CurrentPlanetRequest => {
                let planet_id = self.current_planet.unwrap_or(0);
                let _ = self.tx_orchestrator.send(CurrentPlanetResult {
                    explorer_id: self.id,
                    planet_id,
                });
            }

            NeighborsResponse { neighbors } => {
                if let Some(current) = self.current_planet {
                    self.map.record_neighbors(current, neighbors.clone());
                    // TODO(Vivi): replace with navigation::pick_destination() once implemented
                    if let Some(&dst) = neighbors.first() {
                        let _ = self.tx_orchestrator.send(TravelToPlanetRequest {
                            explorer_id: self.id,
                            current_planet_id: current,
                            dst_planet_id: dst,
                        });
                    }
                }
            }

            BagContentRequest => {
                // TODO(Vivi): replace with bag::JebBag::summarize()
                let _ = self.tx_orchestrator.send(BagContentResponse {
                    explorer_id: self.id,
                    bag_content: vec![],
                });
            }

            SupportedResourceRequest => {
                let _ = self.tx_planet.send(ExplorerToPlanet::SupportedResourceRequest {
                    explorer_id: self.id,
                });
                if let Ok(PlanetToExplorer::SupportedResourceResponse { resource_list }) =
                    self.rx_planet.recv()
                {
                    let _ = self.tx_orchestrator.send(SupportedResourceResult {
                        explorer_id: self.id,
                        supported_resources: resource_list,
                    });
                }
            }

            SupportedCombinationRequest => {
                let _ = self.tx_planet.send(ExplorerToPlanet::SupportedCombinationRequest {
                    explorer_id: self.id,
                });
                if let Ok(PlanetToExplorer::SupportedCombinationResponse { combination_list }) =
                    self.rx_planet.recv()
                {
                    let _ = self.tx_orchestrator.send(SupportedCombinationResult {
                        explorer_id: self.id,
                        combination_list,
                    });
                }
            }

            GenerateResourceRequest { to_generate } => {
                let _ = self.tx_planet.send(ExplorerToPlanet::GenerateResourceRequest {
                    explorer_id: self.id,
                    resource: to_generate,
                });
                match self.rx_planet.recv() {
                    Ok(PlanetToExplorer::GenerateResourceResponse { resource: Some(_res) }) => {
                        // TODO(Vivi): store _res in bag::JebBag
                        let _ = self.tx_orchestrator.send(GenerateResourceResponse {
                            explorer_id: self.id,
                            generated: Ok(()),
                        });
                    }
                    Ok(PlanetToExplorer::GenerateResourceResponse { resource: None }) => {
                        let _ = self.tx_orchestrator.send(GenerateResourceResponse {
                            explorer_id: self.id,
                            generated: Err("planet could not generate the resource".to_string()),
                        });
                    }
                    _ => {}
                }
            }

            CombineResourceRequest { to_generate: _ } => {
                // TODO(Vivi): pull ingredients from bag::JebBag, build ComplexResourceRequest,
                // send to planet via tx_planet, store result back in bag.
                let _ = self.tx_orchestrator.send(CombineResourceResponse {
                    explorer_id: self.id,
                    generated: Err("combine not yet implemented".to_string()),
                });
            }
        }
        true
    }
}
