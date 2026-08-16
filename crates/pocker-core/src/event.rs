//! Event system — typed events with handlers.
//!
//! Events are the extension points of Pocker. There are two kinds:
//! - **Durable events**: persisted to the session log (turn/*, step/*, etc.)
//! - **Live events**: real-time waterfall handlers (agent/pre-step, llm/stream, etc.)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// A typed event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Event name (e.g. "turn/start", "llm/stream", "tool/call")
    pub name: String,
    /// Event payload (JSON)
    pub payload: serde_json::Value,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Whether this is a durable event (persisted) or live (transient)
    pub durable: bool,
}

impl Event {
    pub fn new(name: impl Into<String>, payload: serde_json::Value, durable: bool) -> Self {
        Self {
            name: name.into(),
            payload,
            timestamp: chrono::Utc::now(),
            durable,
        }
    }

    /// Create a durable event (persisted to session log)
    pub fn durable(name: impl Into<String>, payload: serde_json::Value) -> Self {
        Self::new(name, payload, true)
    }

    /// Create a live event (transient, not persisted)
    pub fn live(name: impl Into<String>, payload: serde_json::Value) -> Self {
        Self::new(name, payload, false)
    }
}

/// An event handler function.
pub type EventHandler = Arc<dyn Fn(&Event) + Send + Sync>;

/// A map of event names to their handler lists.
pub struct EventMap {
    handlers: HashMap<String, Vec<EventHandler>>,
}

impl EventMap {
    #[must_use]
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Subscribe a handler to an event.
    pub fn subscribe(&mut self, event_name: &str, handler: EventHandler) {
        self.handlers
            .entry(event_name.to_string())
            .or_default()
            .push(handler);
    }

    /// Emit an event to all subscribers.
    pub fn emit(&self, event: &Event) {
        if let Some(handlers) = self.handlers.get(&event.name) {
            for handler in handlers {
                handler(event);
            }
        }
    }

    /// List all event names that have subscribers.
    #[must_use]
    pub fn list(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }

    /// Count subscribers for a specific event.
    #[must_use]
    pub fn subscriber_count(&self, event_name: &str) -> usize {
        self.handlers.get(event_name).map_or(0, std::vec::Vec::len)
    }
}

impl Default for EventMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn event_creation_durable() {
        let event = Event::durable("turn/start", serde_json::json!({"session": "abc"}));
        assert_eq!(event.name, "turn/start");
        assert!(event.durable);
        assert_eq!(event.payload["session"], "abc");
    }

    #[test]
    fn event_creation_live() {
        let event = Event::live("llm/stream", serde_json::json!({"chunk": "hello"}));
        assert_eq!(event.name, "llm/stream");
        assert!(!event.durable);
    }

    #[test]
    fn event_map_subscribe_and_emit() {
        let mut map = EventMap::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        map.subscribe(
            "test/event",
            Arc::new(move |_event: &Event| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }),
        );

        assert_eq!(map.subscriber_count("test/event"), 1);

        let event = Event::durable("test/event", serde_json::json!({}));
        map.emit(&event);
        map.emit(&event);

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn event_map_no_subscribers() {
        let map = EventMap::new();
        assert_eq!(map.subscriber_count("nonexistent"), 0);

        let event = Event::durable("nonexistent", serde_json::json!({}));
        map.emit(&event); // should not panic
    }

    #[test]
    fn event_map_multiple_subscribers() {
        let mut map = EventMap::new();
        let counter = Arc::new(AtomicUsize::new(0));

        for _ in 0..3 {
            let c = counter.clone();
            map.subscribe(
                "multi/event",
                Arc::new(move |_| {
                    c.fetch_add(1, Ordering::SeqCst);
                }),
            );
        }

        let event = Event::live("multi/event", serde_json::json!({}));
        map.emit(&event);

        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
}
