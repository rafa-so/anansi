//! Pull e push do repositório do workspace (botões da barra superior).

use crate::state::AppState;
use anansi_core::git::{self, GitStatus, PullResult, PushResult};
use anansi_core::{AppError, AppResult};
use std::path::PathBuf;
use tauri::State;

/// O git é um processo externo e o pull/push depende da rede: roda fora das
/// threads do runtime async para não travar outros comandos.
async fn blocking<T: Send + 'static>(
    root: PathBuf,
    f: impl FnOnce(&std::path::Path) -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(move || f(&root))
        .await
        .map_err(|e| AppError::GitError(format!("Falha ao executar o git: {e}")))?
}

#[tauri::command]
pub async fn git_status(state: State<'_, AppState>) -> AppResult<GitStatus> {
    let root = state.workspace()?.root().to_path_buf();
    blocking(root, git::status).await
}

#[tauri::command]
pub async fn git_pull(state: State<'_, AppState>) -> AppResult<PullResult> {
    let root = state.workspace()?.root().to_path_buf();
    blocking(root, git::pull).await
}

/// `message` é o título do commit; só é obrigatório se houver alterações.
#[tauri::command]
pub async fn git_push(
    state: State<'_, AppState>,
    message: Option<String>,
) -> AppResult<PushResult> {
    let root = state.workspace()?.root().to_path_buf();
    blocking(root, move |root| git::push(root, message.as_deref())).await
}
