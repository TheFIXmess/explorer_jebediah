//! # Jebediah's mood
//!
//! Translates game events into log phrases that reflect Jebediah's personality.
//! The GUI reads `Mood::current_phrase()` to show what Jeb is thinking.
//!
//! `From<JebEvent> for &'static str` centralises all phrase mapping so the
//! conversion is defined once and reusable anywhere via `.into()`.

use common_game::utils::ID;

/// Events that change Jebediah's displayed phrase.
#[derive(Debug, Clone)]
pub enum JebEvent {
    Started,
    Arrived(ID),
    Revisited(ID),
    DiscoveredNeighbor(ID),
    AsteroidWhilePresent,
    PlanetDestroyed(ID),
    CollectedResource,
    BagEmpty,
    FullyMapped,
    Killed,
}

impl From<JebEvent> for &'static str {
    fn from(event: JebEvent) -> &'static str {
        match event {
            JebEvent::Started =>
                "Buckle up, galaxy. Jebediah Kerman has arrived.",
            JebEvent::Arrived(_) =>
                "New planet! Never been here. Could be dangerous. LET'S GO.",
            JebEvent::Revisited(_) =>
                "Back again. Still exciting though.",
            JebEvent::DiscoveredNeighbor(_) =>
                "Uncharted territory nearby. Adding it to the list.",
            JebEvent::AsteroidWhilePresent =>
                "Is that an asteroid?! FASCINATING. Taking notes.",
            JebEvent::PlanetDestroyed(_) =>
                "Planet's gone. I saw the whole thing. Worth it.",
            JebEvent::CollectedResource =>
                "Got something. No idea what I'll do with it. Science!",
            JebEvent::BagEmpty =>
                "Bag's empty. Travelled light, as always. No regrets.",
            JebEvent::FullyMapped =>
                "I've been everywhere. Time to go again, faster.",
            JebEvent::Killed =>
                "Tell Valentina I died doing what I loved. Everything.",
        }
    }
}

/// Tracks Jebediah's current phrase and total event count.
pub struct Mood {
    explorer_id: ID,
    current_phrase: &'static str,
    event_count: u32,
}

impl Mood {
    pub fn new(explorer_id: ID) -> Self {
        Self {
            explorer_id,
            current_phrase: "Ready. Strapped in. Extremely eager.",
            event_count: 0,
        }
    }

    /// Updates the phrase for this event and returns it.
    pub fn on_event(&mut self, event: JebEvent) -> &'static str {
        self.event_count += 1;
        self.current_phrase = event.into();
        log::info!("[Jeb #{}] \"{}\"", self.explorer_id, self.current_phrase);
        self.current_phrase
    }

    pub fn current_phrase(&self) -> &'static str {
        self.current_phrase
    }

    pub fn adventure_count(&self) -> u32 {
        self.event_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_converts_event_to_phrase() {
        let phrase: &str = JebEvent::Started.into();
        assert!(phrase.contains("Jebediah"));
    }

    #[test]
    fn mood_tracks_event_count() {
        let mut mood = Mood::new(1);
        mood.on_event(JebEvent::Started);
        mood.on_event(JebEvent::Arrived(3));
        assert_eq!(mood.adventure_count(), 2);
    }

    #[test]
    fn mood_updates_phrase_on_event() {
        let mut mood = Mood::new(1);
        let phrase = mood.on_event(JebEvent::Killed);
        assert!(phrase.contains("Valentina"));
    }
}
