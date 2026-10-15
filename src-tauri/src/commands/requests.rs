use crate::state::AppState;
use anansi_core::model::RequestFile;
use anansi_core::AppResult;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct PathResult {
    path: String,
}

#[tauri::command]
pub async fn request_get(state: State<'_, AppState>, path: String) -> AppResult<RequestFile> {
    state.workspace()?.request_get(&path)
}

#[tauri::command]
pub async fn request_create(
    state: State<'_, AppState>,
    parent_path: String,
    request: RequestFile,
) -> AppResult<PathResult> {
    let path = state.workspace()?.request_create(&parent_path, &request)?;
    Ok(PathResult { path })
}

/// Salva o request; o `path` devolvido muda quando o nome muda de slug.
#[tauri::command]
pub async fn request_save(
    state: State<'_, AppState>,
    path: String,
    request: RequestFile,
) -> AppResult<PathResult> {
    let path = state.workspace()?.request_save(&path, &request)?;
    Ok(PathResult { path })
}

#[tauri::command]
pub async fn request_delete(state: State<'_, AppState>, path: String) -> AppResult<()> {
    state.workspace()?.request_delete(&path)
}

#[tauri::command]
pub async fn request_duplicate(state: State<'_, AppState>, path: String) -> AppResult<PathResult> {
    let path = state.workspace()?.request_duplicate(&path)?;
    Ok(PathResult { path })
}
