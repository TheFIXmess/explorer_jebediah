//! Structured logging utilities for Jebediah explorer operations.
//!
//! Uses the common_game structured logging protocol.

use common_game::logging::{
    ActorType,
    Channel,
    EventType,
    LogEvent,
    Participant,
    Payload,
};
use common_game::utils::ID;

/// Logs a general structured internal explorer event.
pub fn log_explorer_event(
    explorer_id: ID,
    message: impl AsRef<str>,
    channel: Channel,
) {
    let participant = Participant::new(ActorType::Explorer, explorer_id);

    let mut payload = Payload::new();
    payload.insert("message".to_string(), message.as_ref().to_string());

    let event = LogEvent::broadcast(
        participant,
        EventType::InternalExplorerAction,
        channel,
        payload,
    );

    event.emit();
}

/// Logs a message received from the Orchestrator.
pub fn log_orchestrator_message(
    orchestrator_id: ID,
    explorer_id: ID,
    message: impl AsRef<str>,
) {
    let sender = Participant::new(ActorType::Orchestrator, orchestrator_id);
    let receiver = Participant::new(ActorType::Explorer, explorer_id);

    let mut payload = Payload::new();
    payload.insert("message".to_string(), message.as_ref().to_string());

    let event = LogEvent::new(
        Some(sender),
        Some(receiver),
        EventType::MessageOrchestratorToExplorer,
        Channel::Debug,
        payload,
    );

    event.emit();
}

/// Logs a message received from a Planet.
pub fn log_planet_message(
    planet_id: ID,
    explorer_id: ID,
    message: impl AsRef<str>,
) {
    let sender = Participant::new(ActorType::Planet, planet_id);
    let receiver = Participant::new(ActorType::Explorer, explorer_id);

    let mut payload = Payload::new();
    payload.insert("message".to_string(), message.as_ref().to_string());

    let event = LogEvent::new(
        Some(sender),
        Some(receiver),
        EventType::MessagePlanetToExplorer,
        Channel::Debug,
        payload,
    );

    event.emit();
}

/// Logs a message sent from the Explorer to the Orchestrator.
pub fn log_explorer_to_orchestrator_message(
    explorer_id: ID,
    message: impl AsRef<str>,
) {
    let sender = Participant::new(ActorType::Explorer, explorer_id);
    let receiver = Participant::new(ActorType::Orchestrator, 0 as ID);

    let mut payload = Payload::new();
    payload.insert("message".to_string(), message.as_ref().to_string());

    let event = LogEvent::new(
        Some(sender),
        Some(receiver),
        EventType::MessageExplorerToOrchestrator,
        Channel::Debug,
        payload,
    );

    event.emit();
}

/// Logs travel to another planet.
pub fn log_travel(
    explorer_id: ID,
    from_planet: ID,
    to_planet: ID,
    success: bool,
) {
    let channel = if success {
        Channel::Info
    } else {
        Channel::Warning
    };

    let message = if success {
        format!("Successfully traveled from planet {from_planet} to {to_planet}")
    } else {
        format!("Failed to travel from planet {from_planet} to {to_planet}")
    };

    log_explorer_event(explorer_id, message, channel);
}

/// Logs discovered neighbors.
pub fn log_neighbors(
    explorer_id: ID,
    planet_id: ID,
    neighbors: &[ID],
) {
    let message = format!("Discovered neighbors of planet {planet_id}: {neighbors:?}");
    log_explorer_event(explorer_id, message, Channel::Debug);
}

/// Logs bag summary request.
pub fn log_bag_summary(
    explorer_id: ID,
    resource_type_count: usize,
) {
    let message = format!("Bag summary requested: {resource_type_count} resource types");
    log_explorer_event(explorer_id, message, Channel::Debug);
}

/// Logs an attempt to generate a basic resource.
pub fn log_resource_generation_attempt(
    explorer_id: ID,
    resource: impl AsRef<str>,
    success: bool,
) {
    let channel = if success {
        Channel::Info
    } else {
        Channel::Warning
    };

    let message = if success {
        format!("Successfully generated resource {}", resource.as_ref())
    } else {
        format!("Failed to generate resource {}", resource.as_ref())
    };

    log_explorer_event(explorer_id, message, channel);
}

/// Logs an attempt to combine resources into a complex resource.
pub fn log_resource_combination_attempt(
    explorer_id: ID,
    resource: impl AsRef<str>,
    success: bool,
) {
    let channel = if success {
        Channel::Info
    } else {
        Channel::Warning
    };

    let message = if success {
        format!("Successfully combined resources into {}", resource.as_ref())
    } else {
        format!("Failed to combine resources into {}", resource.as_ref())
    };

    log_explorer_event(explorer_id, message, channel);
}