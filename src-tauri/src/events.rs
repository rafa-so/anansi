//! `EventSink` do `anansi-core` implementado com eventos do Tauri.

use anansi_core::events::{Event, EventSink};
use tauri::{AppHandle, Emitter};

pub struct TauriSink(pub AppHandle);

impl EventSink for TauriSink {
    fn emit(&self, event: Event) {
        let _ = self.0.emit(event.name(), &event);
    }
}
