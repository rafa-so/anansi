//! Eventos do backend para o frontend (Seção 5.5 da spec).
//!
//! O `anansi-core` não conhece o Tauri: quem precisa avisar o frontend recebe um
//! `EventSink`. No app ele emite eventos do Tauri; nos testes, guarda em memória.

use serde::Serialize;
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Event {
    /// `workspace://changed`
    WorkspaceChanged { paths: Vec<String> },
    /// `workspace://gitignore`
    Gitignore { ok: bool },
}

impl Event {
    pub fn name(&self) -> &'static str {
        match self {
            Event::WorkspaceChanged { .. } => "workspace://changed",
            Event::Gitignore { .. } => "workspace://gitignore",
        }
    }
}

pub trait EventSink: Send + Sync {
    fn emit(&self, event: Event);
}

/// Implementação para testes: guarda os eventos emitidos.
#[derive(Default)]
pub struct MemorySink {
    events: Mutex<Vec<Event>>,
}

impl MemorySink {
    pub fn take(&self) -> Vec<Event> {
        std::mem::take(&mut self.events.lock().unwrap())
    }
}

impl EventSink for MemorySink {
    fn emit(&self, event: Event) {
        self.events.lock().unwrap().push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_matches_spec() {
        let e = Event::WorkspaceChanged {
            paths: vec!["a/b.json".into()],
        };
        assert_eq!(e.name(), "workspace://changed");
        assert_eq!(
            serde_json::to_value(&e).unwrap(),
            serde_json::json!({ "paths": ["a/b.json"] })
        );
        assert_eq!(
            serde_json::to_value(Event::Gitignore { ok: true }).unwrap(),
            serde_json::json!({ "ok": true })
        );
    }

    #[test]
    fn memory_sink_collects_events() {
        let sink = MemorySink::default();
        sink.emit(Event::Gitignore { ok: false });
        assert_eq!(sink.take(), vec![Event::Gitignore { ok: false }]);
        assert!(sink.take().is_empty());
    }
}
