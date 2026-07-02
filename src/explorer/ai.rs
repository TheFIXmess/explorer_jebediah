//! # Jebediah's core AI
//!
//! State machine for the explorer using the typestate pattern: the current state
//! (`Idle`, `OnPlanet`, `Traveling`) is encoded as a generic type parameter so
//! invalid transitions are caught at compile time rather than at runtime.
//!
//! Jebediah's main priority is fast exploration:
//! - ask neighbors as soon as the AI starts;
//! - travel immediately when a destination is known;
//! - ask neighbors again immediately after every successful move;
//! - resource-related requests are still supported, but movement has priority.

use std::marker::PhantomData;

use common_game::protocols::orchestrator_explorer::{
    ExplorerToOrchestrator,
    OrchestratorToExplorer,
};
use common_game::protocols::planet_explorer::{
    ExplorerToPlanet,
    PlanetToExplorer,
};
use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};

use super::bag::JebBag;
use super::logging;
use super::mapping::GalaxyMap;
use super::mood::{JebEvent, Mood};
use super::navigation;

use crate::BagSummary;

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
pub struct JebExplorerInner<S: State> {
    pub(crate) id: ID,
    pub(crate) rx_orchestrator: Receiver<OrchestratorToExplorer>,
    pub(crate) tx_orchestrator: Sender<ExplorerToOrchestrator<BagSummary>>,

    /// Permanent receiver for planet messages.
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

    fn request_neighbors(&self, current_planet: ID) {
        logging::log_explorer_to_orchestrator_message(
            self.id,
            format!("NeighborsRequest from planet {current_planet}"),
        );

        logging::log_explorer_event(
            self.id,
            format!("[EXPLORE] Requesting neighbors from planet {current_planet}"),
            common_game::logging::Channel::Info,
        );

        let _ = self.tx_orchestrator.send(ExplorerToOrchestrator::NeighborsRequest {
            explorer_id: self.id,
            current_planet_id: current_planet,
        });
    }

    fn send_travel_request(&self, current_planet: ID, destination: ID) {
        if current_planet == destination {
            logging::log_explorer_event(
                self.id,
                format!(
                    "[EXPLORE] Travel ignored because destination equals current planet: {destination}"
                ),
                common_game::logging::Channel::Warning,
            );

            return;
        }

        logging::log_explorer_to_orchestrator_message(
            self.id,
            format!("TravelToPlanetRequest from {current_planet} to {destination}"),
        );

        logging::log_explorer_event(
            self.id,
            format!(
                "[EXPLORE] Requesting travel from planet {current_planet} to planet {destination}"
            ),
            common_game::logging::Channel::Info,
        );

        let _ = self.tx_orchestrator.send(ExplorerToOrchestrator::TravelToPlanetRequest {
            explorer_id: self.id,
            current_planet_id: current_planet,
            dst_planet_id: destination,
        });
    }

    /// Returns `false` when the explorer should stop.
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
                    "Explorer AI started - movement priority enabled",
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

                if let Some(current) = self.current_planet {
                    self.request_neighbors(current);
                }
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
                let current = self.current_planet.unwrap_or(0);

                self.map = GalaxyMap::new(current);
                self.bag = JebBag::new();
                self.mood = Mood::new(self.id);

                logging::log_explorer_event(
                    self.id,
                    format!("Explorer AI reset on planet {current}"),
                    common_game::logging::Channel::Info,
                );

                log::info!("[Jeb #{}] Reset.", self.id);

                let _ = self.tx_orchestrator.send(ResetExplorerAIResult {
                    explorer_id: self.id,
                });

                if current != 0 {
                    self.request_neighbors(current);
                }
            }

            MoveToPlanet {
                sender_to_new_planet,
                planet_id,
            } => {
                let from = self.current_planet.unwrap_or(planet_id);

                match sender_to_new_planet {
                    Some(new_tx) => {
                        self.tx_planet = new_tx;
                        self.current_planet = Some(planet_id);
                        self.map.visit(planet_id);

                        logging::log_travel(
                            self.id,
                            from,
                            planet_id,
                            true,
                        );

                        logging::log_explorer_event(
                            self.id,
                            format!(
                                "[EXPLORE] Arrived on planet {planet_id}. Asking neighbors immediately."
                            ),
                            common_game::logging::Channel::Info,
                        );

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

                        self.request_neighbors(planet_id);
                    }

                    None => {
                        logging::log_travel(
                            self.id,
                            from,
                            planet_id,
                            false,
                        );

                        logging::log_explorer_event(
                            self.id,
                            format!(
                                "[EXPLORE] Failed to move from planet {from} to planet {planet_id}. Staying on planet {from}."
                            ),
                            common_game::logging::Channel::Warning,
                        );

                        let _ = self.tx_orchestrator.send(MovedToPlanetResult {
                            explorer_id: self.id,
                            planet_id,
                        });

                        if let Some(current) = self.current_planet {
                            self.request_neighbors(current);
                        }
                    }
                }
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
                let Some(current) = self.current_planet else {
                    logging::log_explorer_event(
                        self.id,
                        "NeighborsResponse received but current planet is unknown",
                        common_game::logging::Channel::Warning,
                    );

                    return true;
                };

                self.map.record_neighbors(current, neighbors.clone());

                logging::log_neighbors(
                    self.id,
                    current,
                    &neighbors,
                );

                logging::log_explorer_event(
                    self.id,
                    format!(
                        "[EXPLORE] Map updated from planet {current}. {}",
                        self.map.log_summary()
                    ),
                    common_game::logging::Channel::Info,
                );

                for neighbor in &neighbors {
                    if self.map.visit_count(*neighbor) == 0 {
                        let _ = self
                            .mood
                            .on_event(JebEvent::DiscoveredNeighbor(*neighbor));
                    }
                }

                match navigation::pick_destination(current, &self.map) {
                    Some(destination) => {
                        logging::log_explorer_event(
                            self.id,
                            format!(
                                "[EXPLORE] Navigation selected destination {destination} from planet {current}"
                            ),
                            common_game::logging::Channel::Info,
                        );

                        self.send_travel_request(current, destination);
                    }

                    None => {
                        logging::log_explorer_event(
                            self.id,
                            format!(
                                "[EXPLORE] No destination available from planet {current}. {}",
                                self.map.log_summary()
                            ),
                            common_game::logging::Channel::Info,
                        );

                        let _ = self.mood.on_event(JebEvent::FullyMapped);
                    }
                }
            }

            BagContentRequest => {
                let summary = self.bag.summarize();

                logging::log_bag_summary(
                    self.id,
                    summary.len(),
                );

                let _ = self.tx_orchestrator.send(BagContentResponse {
                    explorer_id: self.id,
                    bag_content: summary,
                });
            }

            SupportedResourceRequest => {
                logging::log_explorer_event(
                    self.id,
                    "Manual SupportedResourceRequest received",
                    common_game::logging::Channel::Debug,
                );

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

                if let Some(current) = self.current_planet {
                    self.request_neighbors(current);
                }
            }

            SupportedCombinationRequest => {
                logging::log_explorer_event(
                    self.id,
                    "Manual SupportedCombinationRequest received",
                    common_game::logging::Channel::Debug,
                );

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

                if let Some(current) = self.current_planet {
                    self.request_neighbors(current);
                }
            }

            GenerateResourceRequest { to_generate } => {
                logging::log_explorer_event(
                    self.id,
                    format!("Manual GenerateResourceRequest received: {to_generate:?}"),
                    common_game::logging::Channel::Debug,
                );

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
                            PlanetToExplorer::GenerateResourceResponse {
                                resource: Some(res),
                            } => {
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

                            PlanetToExplorer::GenerateResourceResponse {
                                resource: None,
                            } => {
                                logging::log_resource_generation_attempt(
                                    self.id,
                                    format!("{to_generate:?}"),
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
                            format!("{to_generate:?}"),
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

                if let Some(current) = self.current_planet {
                    self.request_neighbors(current);
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

                if let Some(current) = self.current_planet {
                    self.request_neighbors(current);
                }
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

            PlanetToExplorer::Stopped { .. } => "Stopped",
        }
    }
}