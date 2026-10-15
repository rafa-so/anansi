//! Estado global do app.

use anansi_core::http::Executor;
use anansi_core::watcher::WorkspaceWatcher;
use anansi_core::{AppError, AppResult, Workspace};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct AppState {
    workspace: Mutex<Option<Workspace>>,
    watcher: Mutex<Option<WorkspaceWatcher>>,
    pub executor: Arc<Executor>,
}

impl AppState {
    /// Cópia do workspace aberto (o `Workspace` só guarda a raiz).
    pub fn workspace(&self) -> AppResult<Workspace> {
        self.workspace
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(AppError::workspace_not_open)
    }

    pub fn has_workspace(&self) -> bool {
        self.workspace.lock().unwrap().is_some()
    }

    /// Troca o workspace aberto; o watcher anterior para ao ser descartado.
    pub fn set_workspace(&self, ws: Workspace, watcher: Option<WorkspaceWatcher>) {
        *self.watcher.lock().unwrap() = watcher;
        *self.workspace.lock().unwrap() = Some(ws);
    }
}
