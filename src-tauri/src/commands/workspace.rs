use crate::events::TauriSink;
use crate::{prefs, state::AppState};
use anansi_core::model::{TreeNode, WorkspaceInfo};
use anansi_core::watcher::WorkspaceWatcher;
use anansi_core::{AppResult, Workspace};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, State};

fn activate(app: &AppHandle, state: &AppState, ws: Workspace) -> AppResult<WorkspaceInfo> {
    let info = ws.info()?;
    prefs::set(
        app,
        prefs::LAST_WORKSPACE,
        serde_json::Value::from(info.path.clone()),
    );
    // Sem watcher (ex.: unidade de rede), o "Recarregar workspace" continua funcionando.
    let watcher = match WorkspaceWatcher::start(ws.root(), Arc::new(TauriSink(app.clone()))) {
        Ok(w) => Some(w),
        Err(e) => {
            eprintln!("{e}");
            None
        }
    };
    state.set_workspace(ws, watcher);
    Ok(info)
}

/// Abre uma pasta que já é workspace. Sem `workspace.json`, retorna
/// `NOT_A_WORKSPACE` e o frontend oferece inicializar.
#[tauri::command]
pub async fn workspace_open(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> AppResult<WorkspaceInfo> {
    let ws = Workspace::open(Path::new(&path))?;
    activate(&app, &state, ws)
}

#[tauri::command]
pub async fn workspace_init(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> AppResult<WorkspaceInfo> {
    let ws = Workspace::init(Path::new(&path))?;
    activate(&app, &state, ws)
}

/// Workspace aberto, ou o último usado (reaberto em silêncio). `null` se nenhum.
#[tauri::command]
pub async fn workspace_current(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<Option<WorkspaceInfo>> {
    if state.has_workspace() {
        return state.workspace()?.info().map(Some);
    }
    let candidate = prefs::e2e_workspace().or_else(|| {
        prefs::get(&app, prefs::LAST_WORKSPACE).and_then(|v| v.as_str().map(PathBuf::from))
    });
    let Some(path) = candidate else {
        return Ok(None);
    };
    match Workspace::open(&path) {
        Ok(ws) => activate(&app, &state, ws).map(Some),
        Err(_) => Ok(None),
    }
}

#[tauri::command]
pub async fn workspace_tree(state: State<'_, AppState>) -> AppResult<Vec<TreeNode>> {
    Ok(state.workspace()?.tree())
}

/// Quantos requests a remoção do item apaga (para o texto da confirmação).
#[tauri::command]
pub async fn item_request_count(state: State<'_, AppState>, path: String) -> AppResult<usize> {
    state.workspace()?.request_count(&path)
}
