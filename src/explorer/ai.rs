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

use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator,
    OrchestratorToExplorer,
};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};

use super::bag::JebBag;
use super::logging;
use super::mapping::GalaxyMap;
use super::mood::{JebEvent, Mood};
use super::navigation;

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
    pub(crate) bag: JebBag,
    pub(crate) mood: Mood,
    pub(crate) _state: PhantomData<S>,
    pub(crate) current_planet: Option<ID>,
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
            bag: JebBag::new(),
            mood: Mood::new(id),
            _state: PhantomData,
            current_planet: Some(starting_planet),
        })
    }

    /// Blocks on the orchestrator channel, handling messages until killed.
    pub fn run(mut self) -> Result<(), String> {
        logging::log_explorer_event(
            self.id,
            "Jebediah engine started",
            common_game::logging::Channel::Info,
        );

        log::info!(
            "[Jeb #{}] Engines on. Let's see what's out there.",
            self.id
        );

        loop {
            match self.rx_orchestrator.recv() {
                Ok(msg) => {
                    if !self.handle_orchestrator_message(msg) {
                        break;
                    }
                }
                Err(_) => {
                    logging::log_explorer_event(
                        self.id,
                        "Orchestrator channel closed",
                        common_game::logging::Channel::Warning,
                    );

                    log::warn!("[Jeb #{}] Orchestrator channel closed.", self.id);
                    break;
                }
            }
        }

        logging::log_explorer_event(
            self.id,
            "Jebediah going dark",
            common_game::logging::Channel::Info,
        );

        log::info!("[Jeb #{}] Going dark.", self.id);
        Ok(())
    }

    /// Returns `false` when the explorer should stop, for example on `KillExplorer`.
    fn handle_orchestrator_message(&mut self, msg: OrchestratorToExplorer) -> bool {
        use ExplorerToOrchestrator::*;
        use OrchestratorToExplorer::*;

        logging::log_orchestrator_message(
            0,
            self.id,
            Self::orchestrator_message_name(&msg),
        );

        match msg {
            StartExplorerAI => {
                logging::log_explorer_event(
                    self.id,
                    "Explorer AI started",
                    common_game::logging::Channel::Info,
                );

                log::info!(
                    "[Jeb #{}] {}",
                    self.id,
                    self.mood.on_event(JebEvent::Started)
                );

                let _ = self.tx_orchestrator.send(StartExplorerAIResult {
                    explorer_id: self.id,
                });
            }

            StopExplorerAI => {
                logging::log_explorer_event(
                    self.id,
                    "Explorer AI stopped",
                    common_game::logging::Channel::Info,
                );

                log::info!("[Jeb #{}] Stopping.", self.id);

                let _ = self.tx_orchestrator.send(StopExplorerAIResult {
                    explorer_id: self.id,
                });
            }

            KillExplorer => {
                logging::log_explorer_event(
                    self.id,
                    "Explorer killed",
                    common_game::logging::Channel::Info,
                );

                log::info!(
                    "[Jeb #{}] {}",
                    self.id,
                    self.mood.on_event(JebEvent::Killed)
                );

                let _ = self.tx_orchestrator.send(KillExplorerResult {
                    explorer_id: self.id,
                });

                return false;
            }

            ResetExplorerAI => {
                self.map = GalaxyMap::new(self.current_planet.unwrap_or(0));
                self.bag = JebBag::new();
                self.mood = Mood::new(self.id);

                logging::log_explorer_event(
                    self.id,
                    "Explorer AI reset",
                    common_game::logging::Channel::Info,
                );

                log::info!("[Jeb #{}] Reset.", self.id);

                let _ = self.tx_orchestrator.send(ResetExplorerAIResult {
                    explorer_id: self.id,
                });
            }

            MoveToPlanet {
                sender_to_new_planet,
                planet_id,
            } => {
                let from = self.current_planet.unwrap_or(planet_id);

                if let Some(new_tx) = sender_to_new_planet {
                    self.tx_planet = new_tx;
                }

                self.current_planet = Some(planet_id);
                self.map.visit(planet_id);

                logging::log_travel(self.id, from, planet_id, true);

                log::info!(
                    "[Jeb #{}] {} Arrived at planet {}.",
                    self.id,
                    self.mood.on_event(JebEvent::Arrived(planet_id)),
                    planet_id
                );

                let _ = self.tx_orchestrator.send(MovedToPlanetResult {
                    explorer_id: self.id,
                    planet_id,
                });
            }

            CurrentPlanetRequest => {
                let planet_id = self.current_planet.unwrap_or(0);

                logging::log_explorer_to_orchestrator_message(
                    self.id,
                    format!("CurrentPlanetResult: {planet_id}"),
                );

                let _ = self.tx_orchestrator.send(CurrentPlanetResult {
                    explorer_id: self.id,
                    planet_id,
                });
            }

            NeighborsResponse { neighbors } => {
                if let Some(current) = self.current_planet {
                    self.map.record_neighbors(current, neighbors.clone());

                    logging::log_neighbors(self.id, current, &neighbors);

                    for neighbor in &neighbors {
                        if self.map.visit_count(*neighbor) == 0 {
                            let _ = self.mood.on_event(JebEvent::DiscoveredNeighbor(*neighbor));
                        }
                    }

                    if let Some(dst) = navigation::pick_destination(current, &self.map) {
                        logging::log_explorer_to_orchestrator_message(
                            self.id,
                            format!("TravelToPlanetRequest from {current} to {dst}"),
                        );

                        let _ = self.tx_orchestrator.send(TravelToPlanetRequest {
                            explorer_id: self.id,
                            current_planet_id: current,
                            dst_planet_id: dst,
                        });
                    } else {
                        logging::log_explorer_event(
                            self.id,
                            "No destination available from current planet",
                            common_game::logging::Channel::Debug,
                        );

                        let _ = self.mood.on_event(JebEvent::FullyMapped);
                    }
                }
            }

            BagContentRequest => {
                let summary = self.bag.summarize();

                logging::log_bag_summary(self.id, summary.len());

                let _ = self.tx_orchestrator.send(BagContentResponse {
                    explorer_id: self.id,
                    bag_content: summary,
                });
            }

            SupportedResourceRequest => {
                let _ = self.tx_planet.send(ExplorerToPlanet::SupportedResourceRequest {
                    explorer_id: self.id,
                });

                match self.rx_planet.recv() {
                    Ok(msg) => {
                        let planet_id = self.current_planet.unwrap_or(0);

                        logging::log_planet_message(
                            planet_id,
                            self.id,
                            Self::planet_message_name(&msg),
                        );

                        if let PlanetToExplorer::SupportedResourceResponse { resource_list } = msg {
                            let _ = self.tx_orchestrator.send(SupportedResourceResult {
                                explorer_id: self.id,
                                supported_resources: resource_list,
                            });
                        }
                    }
                    Err(_) => {
                        logging::log_explorer_event(
                            self.id,
                            "Failed to receive SupportedResourceResponse from planet",
                            common_game::logging::Channel::Warning,
                        );
                    }
                }
            }

            SupportedCombinationRequest => {
                let _ = self
                    .tx_planet
                    .send(ExplorerToPlanet::SupportedCombinationRequest {
                        explorer_id: self.id,
                    });

                match self.rx_planet.recv() {
                    Ok(msg) => {
                        let planet_id = self.current_planet.unwrap_or(0);

                        logging::log_planet_message(
                            planet_id,
                            self.id,
                            Self::planet_message_name(&msg),
                        );

                        if let PlanetToExplorer::SupportedCombinationResponse {
                            combination_list,
                        } = msg
                        {
                            let _ = self.tx_orchestrator.send(SupportedCombinationResult {
                                explorer_id: self.id,
                                combination_list,
                            });
                        }
                    }
                    Err(_) => {
                        logging::log_explorer_event(
                            self.id,
                            "Failed to receive SupportedCombinationResponse from planet",
                            common_game::logging::Channel::Warning,
                        );
                    }
                }
            }

            GenerateResourceRequest { to_generate } => {
                let _ = self.tx_planet.send(ExplorerToPlanet::GenerateResourceRequest {
                    explorer_id: self.id,
                    resource: to_generate,
                });

                match self.rx_planet.recv() {
                    Ok(msg) => {
                        let planet_id = self.current_planet.unwrap_or(0);

                        logging::log_planet_message(
                            planet_id,
                            self.id,
                            Self::planet_message_name(&msg),
                        );

                        match msg {
                            PlanetToExplorer::GenerateResourceResponse { resource: Some(res) } => {
                                logging::log_resource_generation_attempt(
                                    self.id,
                                    format!("{:?}", res.get_type()),
                                    true,
                                );

                                self.bag.add_basic(res);
                                let _ = self.mood.on_event(JebEvent::CollectedResource);

                                let _ = self.tx_orchestrator.send(GenerateResourceResponse {
                                    explorer_id: self.id,
                                    generated: Ok(()),
                                });
                            }

                            PlanetToExplorer::GenerateResourceResponse { resource: None } => {
                                logging::log_resource_generation_attempt(
                                    self.id,
                                    "unknown",
                                    false,
                                );

                                let _ = self.tx_orchestrator.send(GenerateResourceResponse {
                                    explorer_id: self.id,
                                    generated: Err(
                                        "planet could not generate the resource".to_string(),
                                    ),
                                });
                            }

                            other => {
                                logging::log_explorer_event(
                                    self.id,
                                    format!(
                                        "Unexpected planet response while generating resource: {}",
                                        Self::planet_message_name(&other)
                                    ),
                                    common_game::logging::Channel::Warning,
                                );
                            }
                        }
                    }
                    Err(_) => {
                        logging::log_resource_generation_attempt(
                            self.id,
                            "unknown",
                            false,
                        );

                        let _ = self.tx_orchestrator.send(GenerateResourceResponse {
                            explorer_id: self.id,
                            generated: Err(
                                "failed to receive GenerateResourceResponse from planet"
                                    .to_string(),
                            ),
                        });
                    }
                }
            }

            CombineResourceRequest { to_generate } => {
                logging::log_resource_combination_attempt(
                    self.id,
                    format!("{to_generate:?}"),
                    false,
                );

                let _ = self.tx_orchestrator.send(CombineResourceResponse {
                    explorer_id: self.id,
                    generated: Err("combine not yet implemented".to_string()),
                });
            }
        }

        true
    }

    fn orchestrator_message_name(msg: &OrchestratorToExplorer) -> &'static str {
        match msg {
            OrchestratorToExplorer::StartExplorerAI => "StartExplorerAI",
            OrchestratorToExplorer::StopExplorerAI => "StopExplorerAI",
            OrchestratorToExplorer::KillExplorer => "KillExplorer",
            OrchestratorToExplorer::ResetExplorerAI => "ResetExplorerAI",
            OrchestratorToExplorer::MoveToPlanet { .. } => "MoveToPlanet",
            OrchestratorToExplorer::CurrentPlanetRequest => "CurrentPlanetRequest",
            OrchestratorToExplorer::NeighborsResponse { .. } => "NeighborsResponse",
            OrchestratorToExplorer::BagContentRequest => "BagContentRequest",
            OrchestratorToExplorer::SupportedResourceRequest => "SupportedResourceRequest",
            OrchestratorToExplorer::SupportedCombinationRequest => {
                "SupportedCombinationRequest"
            }
            OrchestratorToExplorer::GenerateResourceRequest { .. } => {
                "GenerateResourceRequest"
            }
            OrchestratorToExplorer::CombineResourceRequest { .. } => {
                "CombineResourceRequest"
            }
        }
    }

    fn planet_message_name(msg: &PlanetToExplorer) -> &'static str {
        match msg {
            PlanetToExplorer::SupportedResourceResponse { .. } => {
                "SupportedResourceResponse"
            }
            PlanetToExplorer::SupportedCombinationResponse { .. } => {
                "SupportedCombinationResponse"
            }
            PlanetToExplorer::GenerateResourceResponse { .. } => {
                "GenerateResourceResponse"
            }
            PlanetToExplorer::CombineResourceResponse { .. } => {
                "CombineResourceResponse"
            }
            PlanetToExplorer::AvailableEnergyCellResponse { .. } => {
                "AvailableEnergyCellResponse"
            }
            PlanetToExplorer::Stopped { .. } => "Stopped"
        }
    }
}