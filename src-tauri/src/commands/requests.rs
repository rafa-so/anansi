use crate::state::AppState;
use anansi_core::env::{bearer, SecretAction};
use anansi_core::model::RequestFile;
use anansi_core::workspace::{MovedRequest, SavedRequest};
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

/// Cria o request. Um token Bearer literal vai para o segredo `bearer_token` de `env`.
#[tauri::command]
pub async fn request_create(
    state: State<'_, AppState>,
    parent_path: String,
    request: RequestFile,
    env: Option<String>,
) -> AppResult<SavedRequest> {
    state
        .workspace()?
        .request_create_in_env(&parent_path, &request, env.as_deref())
}

/// Salva o request; o `path` devolvido muda quando o nome muda de slug, e o
/// `request` devolvido é o que foi gravado (com `{{bearer_token}}` no lugar do token).
#[tauri::command]
pub async fn request_save(
    state: State<'_, AppState>,
    path: String,
    request: RequestFile,
    env: Option<String>,
) -> AppResult<SavedRequest> {
    state
        .workspace()?
        .request_save_in_env(&path, &request, env.as_deref())
}

#[tauri::command]
pub async fn request_delete(state: State<'_, AppState>, path: String) -> AppResult<()> {
    state.workspace()?.request_delete(&path)
}

#[tauri::command]
pub async fn request_duplicate(
    state: State<'_, AppState>,
    path: String,
    env: Option<String>,
) -> AppResult<PathResult> {
    let path = state
        .workspace()?
        .request_duplicate(&path, env.as_deref())?;
    Ok(PathResult { path })
}

/// Move o request para outra pasta do workspace (docs/mover-request).
/// `renamed` indica que o nome do arquivo ganhou sufixo por conflito no destino.
#[tauri::command]
pub async fn request_move(
    state: State<'_, AppState>,
    path: String,
    target_parent: String,
) -> AppResult<MovedRequest> {
    state.workspace()?.request_move(&path, &target_parent)
}

/// Prévia do "Importar cURL": o que acontece com o segredo `bearer_token`
/// (`null` se o request não tem token literal).
#[tauri::command]
pub async fn bearer_preview(
    state: State<'_, AppState>,
    request: RequestFile,
    env: Option<String>,
) -> AppResult<Option<SecretAction>> {
    bearer::preview(state.workspace()?.root(), &request, env.as_deref())
}
